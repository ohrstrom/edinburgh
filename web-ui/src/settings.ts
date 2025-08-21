const settings = {
  FRAME_FORWARDER_ENDPOINT: import.meta.env?.VITE_FRAME_FORWARDER_ENDPOINT ?? '/ws',
  ENSEMBLE_DIRECTORY_ENDPOINT: import.meta.env?.VITE_ENSEMBLE_DIRECTORY_ENDPOINT ?? '/ensembles',
}

console.log('settings', settings)

export default settings
