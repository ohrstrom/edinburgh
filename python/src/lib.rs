use pyo3::prelude::*;
use pyo3::types::PyBytes;
use shared::edi::EDISource;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

type PyCallback = PyObject;

#[pyclass]
struct EDI {
    _inner: Arc<Mutex<EDISource>>,
    _callbacks: Arc<Mutex<HashMap<String, Vec<PyCallback>>>>,
    tx: Sender<Vec<u8>>,
    _rt: Arc<Runtime>,
}

#[pymethods]
impl EDI {
    #[new]
    fn new(_py: Python<'_>) -> PyResult<Self> {
        let source = Arc::new(Mutex::new(EDISource::new(None, None, None)));
        let callbacks = Arc::new(Mutex::new(HashMap::new()));

        // let source_clone = source.clone();
        // let callbacks_clone = callbacks.clone();

        let rt = Arc::new(Runtime::new()?);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

        rt.spawn(async move {
            let mut edisource = EDISource::new(None, None, None);
            while let Some(data) = rx.recv().await {
                let _ = edisource.feed(&data).await;
            }
        });

        Ok(EDI {
            _inner: source,
            _callbacks: callbacks,
            tx,
            _rt: rt,
        })
    }

    fn feed(&self, _py: Python<'_>, data: Bound<'_, PyBytes>) -> PyResult<()> {
        let data = data.as_bytes().to_vec();

        match self.tx.try_send(data) {
            Ok(_) => Ok(()),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Channel error: {}",
                e
            ))),
        }
    }

    fn reset(&self) -> PyResult<()> {
        // self.inner.lock().unwrap().reset();
        Ok(())
    }
}

#[pymodule]
fn edinburgh(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<EDI>()?;
    Ok(())
}
