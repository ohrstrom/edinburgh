<script>
    import MP4Box from "@webav/mp4box.js";
  
    let ws;
    let mediaSource;
    let sourceBuffer;
    let mp4boxfile;
    let audioElement;
    let trackId;
    let appendQueue = [];
    let processing = false;
  
    const connect = async () => {
  
        if (!ws) {
            ws = new WebSocket("ws://localhost:9001");
  
            ws.binaryType = "arraybuffer";
    
            ws.onmessage = (event) => {
                const rawAACPFrame = new Uint8Array(event.data);
                handleAACPFrame(rawAACPFrame);
            };
        }
        console.debug("ws:", ws);
    };
  
    const handleAACPFrame = (AACPFrame) => {
        if (!mp4boxfile) initializeMP4Box();
  
        // let sample = {
        //     data: AACPFrame,
        //     duration: 1024,  // AAC frame duration in samples (common value for 44.1kHz)
        //     dts: performance.now() * 90, // Rough DTS in 90kHz clock
        //     cts: 0,
        //     is_sync: true
        // };
  
        // mp4boxfile.addSample(trackId, sample);
  
        let buffer = AACPFrame.buffer;
        buffer.fileStart = 0;
  
        mp4boxfile.appendBuffer(buffer);
  
        mp4boxfile.flush();
  
        // console.debug("mp4F:", trackId);
  
    };
  
    const initializeMP4Box = () => {
        mp4boxfile = MP4Box.createFile();
        mediaSource = new MediaSource();
  
        audioElement = document.getElementById("audioPlayer");
        audioElement.src = URL.createObjectURL(mediaSource);
  
        mediaSource.addEventListener("sourceopen", () => {
            console.log("MediaSource opened");
  
            // Define the track
            trackId = mp4boxfile.addTrack({
                timescale: 90000,  // MPEG TS timebase
                codec: "mp4a.40.0", // AAC-LC / HE-AAC
                duration: 0
            });
  
            mp4boxfile.onError = function(e) {
                console.error("MP4Box onError:", e);
            };
  
            mp4boxfile.onReady = (info) => {
                console.log("MP4Box onReady:", info);
                // sourceBuffer = mediaSource.addSourceBuffer(info.tracks[0].codec);
                // sourceBuffer.mode = "sequence";
  
                // sourceBuffer.addEventListener("updateend", processAppendQueue);
  
                mp4boxfile.onSegment = function (id, user, buffer, sampleNumber, last) {
                    console.log("MP4Box onSegment:", id);
                    // appendQueue.push(buffer);
                    // processAppendQueue();
                };
                mp4boxfile.setSegmentOptions(info.tracks[0].id, sb, options);
                let initSegs = mp4boxfile.initializeSegmentation();  
                console.debug("initSegs:", initSegs);
                mp4boxfile.start();
            };
  
            // mp4boxfile.onSamples = function (id, user, samples) {
            //     console.log("MP4Box onSamples:", id);
            // };
  
            // mp4boxfile.onSegment = (id, user, buffer, sampleNum) => {
            //   console.log("MP4Box onSegment:", id);
            //     // appendQueue.push(buffer);
            //     // processAppendQueue();
            // };
  
            // mp4boxfile.start();
        });
    };
  
  </script>
    
  <main>
      <div>
          <h1>AAC Stream (MP4 Encapsulation)</h1>
          <button on:click={connect}>Connect</button>
      </div>
      <audio id="audioPlayer" controls></audio>
  </main>
    