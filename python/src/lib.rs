use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use shared::edi::bus::{init_event_bus, EDIEvent};
use shared::edi::EDISource;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc::{Sender, UnboundedReceiver};
use tokio::sync::Mutex;

type PyCallback = PyObject;

#[pyclass]
struct EDI {
    _inner: Arc<Mutex<EDISource>>,
    _callbacks: Arc<Mutex<HashMap<String, Vec<PyCallback>>>>,
    tx: Sender<Vec<u8>>,
    // keep runtime alive for the life of the object
    _rt: Arc<Runtime>,
}

#[pymethods]
impl EDI {
    #[new]
    fn new(_py: Python<'_>) -> PyResult<Self> {
        // Build a runtime with I/O and timers enabled (good general default)
        let rt = Arc::new(
            Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?,
        );

        let source = Arc::new(Mutex::new(EDISource::new(None, None, None)));
        let callbacks = Arc::new(Mutex::new(HashMap::new()));

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

        // spawn feed loop on OUR runtime
        {
            let handle = rt.handle().clone();
            handle.spawn(async move {
                let mut edisource = EDISource::new(None, None, None);
                while let Some(data) = rx.recv().await {
                    let _ = edisource.feed(&data).await;
                }
            });
        }

        // init the bus and spawn the event handler on OUR runtime
        let edi_rx = init_event_bus();
        let event_handler = EDIHandler::new(edi_rx, callbacks.clone());

        {
            let handle = rt.handle().clone();
            handle.spawn(async move {
                event_handler.run().await;
            });
        }

        Ok(EDI {
            _inner: source,
            _callbacks: callbacks,
            tx,
            _rt: rt,
        })
    }

    fn feed(&self, _py: Python<'_>, data: Bound<'_, PyBytes>) -> PyResult<()> {
        match self.tx.try_send(data.as_bytes().to_vec()) {
            Ok(_) => Ok(()),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Channel error: {e}"
            ))),
        }
    }

    fn reset(&self) -> PyResult<()> {
        Ok(())
    }
}

struct EDIHandler {
    edi_rx: UnboundedReceiver<EDIEvent>,
    callbacks: Arc<Mutex<HashMap<String, Vec<PyCallback>>>>,
}

impl EDIHandler {
    pub fn new(
        edi_rx: UnboundedReceiver<EDIEvent>,
        callbacks: Arc<Mutex<HashMap<String, Vec<PyCallback>>>>,
    ) -> Self {
        Self { edi_rx, callbacks }
    }

    pub async fn run(mut self) {
        while let Some(event) = self.edi_rx.recv().await {
            match event {
                EDIEvent::EnsembleUpdated(ensemble) => {
                    println!("Ensemble updated: {:?}", ensemble);
                }
                EDIEvent::MOTImageReceived(m) => {
                    println!("MOT Image received: {:?}", m);
                }
                EDIEvent::DLObjectReceived(d) => {
                    println!("DL Object received: {:?}", d);
                }
                _ => (),
            }
        }
    }
    fn emit<F>(&self, event: &str, build_payload: F)
    where
        F: for<'py> FnOnce(Python<'py>) -> PyObject,
    {
        Python::with_gil(|py| {
            let callbacks: Vec<PyCallback> = {
                let map = self.callbacks.blocking_lock();
                map.get(event)
                    .map(|v| v.iter().map(|c| c.clone_ref(py)).collect())
                    .unwrap_or_default()
            };

            if callbacks.is_empty() {
                return;
            }

            let payload = build_payload(py);

            let inspect = py.import("inspect").ok();
            let asyncio = py.import("asyncio").ok();
            let loop_obj = asyncio
                .as_ref()
                .and_then(|a| a.call_method0("get_running_loop").ok());

            for cb in callbacks {
                match cb.call1(py, (payload.clone_ref(py),)) {
                    Ok(ret) => {
                        let is_awaitable = inspect
                            .as_ref()
                            .and_then(|ins| {
                                ins.getattr("isawaitable")
                                    .ok()
                                    .and_then(|f| f.call1((ret.clone_ref(py),)).ok())
                                    .and_then(|b| b.extract::<bool>().ok())
                            })
                            .unwrap_or(false);

                        if is_awaitable {
                            if let Some(loop_obj) = loop_obj.as_ref() {
                                let _ = loop_obj.call_method1("create_task", (ret,));
                            } else if let Some(asyncio) = asyncio.as_ref() {
                                let _ = asyncio.call_method1("create_task", (ret,));
                            }
                        }
                    }
                    Err(e) => e.print(py),
                }
            }
        });
    }
}

#[pymodule]
fn edinburgh(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<EDI>()?;
    Ok(())
}
