let instance = null
let initialized = false

import Faad2Module from './faad2.js'

export async function initDecoder(ascBytes) {
    if (!instance) {
        // const mod = await import('./faad2.js')
        // instance = await mod.default
        // console.debug("instance", mod, instance)
        instance = await Faad2Module()
    }

    console.debug("capabilities", instance._get_faad_capabilities())

    const ascPtr = instance._malloc(ascBytes.length)
    instance.HEAPU8.set(ascBytes, ascPtr)

    const result = instance._init_decoder(ascPtr, ascBytes.length)
    instance._free(ascPtr)

    console.debug('init result:', result)

    if (result < 0) throw new Error('Failed to init FAAD decoder')
    initialized = true
}

export function decodeAAC(frameBytes) {
    if (!instance || !initialized) throw new Error('Decoder not initialized')

    const inPtr = instance._malloc(frameBytes.length)
    const outPtr = instance._malloc(4096 * 4 * 2) // max output buffer size

    instance.HEAPU8.set(frameBytes, inPtr)
    const samples = instance._decode_frame(inPtr, frameBytes.length, outPtr, 4096 * 4 * 2)

    instance._free(inPtr)

    if (samples <= 0) {
        instance._free(outPtr)
        return null // or throw, or skip
    }

    const numChannels = 2
    const numFrames = samples / numChannels

    const raw = new Float32Array(instance.HEAPU8.buffer, outPtr, samples)
    const pcmData = [
        new Float32Array(numFrames),
        new Float32Array(numFrames),
    ]

    for (let i = 0; i < numFrames; i++) {
        pcmData[0][i] = raw[i * 2]     // Left
        pcmData[1][i] = raw[i * 2 + 1] // Right
    }

    instance._free(outPtr)

    return pcmData
}
