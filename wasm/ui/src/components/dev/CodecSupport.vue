<script lang="ts" setup>
import { ComputedRef, Ref, ref, watch } from 'vue'

const codecs = [
    {
        name: 'AAC-HE v1', kind: 'mp4a.40.5', asc: [0x13, 0x14, 0x56, 0xe5, 0x98],
    },
    {
        name: 'AAC-HE v2', kind: 'mp4a.40.29', asc: [0x14, 0x0C, 0x56, 0xE5, 0xAD, 0x48, 0x80],
    },
]


const probeDecoder = async (codec) => {
    const kind = codec.kind
    const asc = new Uint8Array(codec.asc)

    const config = {
        codec: kind,
        sampleRate: 48_000,
        numberOfChannels: 2,
        description: asc,
    }

    if (!AudioDecoder) {
        console.error('AudioDecoder is not supported in this environment.')
        return false
    }

    let supported = false
    let error = null

    try {
        const res = await AudioDecoder.isConfigSupported(config)
        if (res.supported) {
            supported = true
            console.log('Supported codec:', kind, res.supported, '--------')
        } else {
            console.log('Unsupported codec:', kind)
            error = 'Unsupported codec'
        }
    } catch (err) {
        console.error('Error checking codec support:', err);
        error = 'Error checking codec support';
    }

    const decoder = new AudioDecoder({
        output: (ad) => console.debug("decoded", ad),
        error: (err) => {
            console.info('Decoder error:', err);
            error = `err: ${err.name} ${err.message}`;
        }
    });

    try {
        await decoder.configure(config)
        console.log('Configured decoder:', kind)
    } catch (err) {
        console.error('Error configuring decoder:', err)
        error = 'Error configuring decoder'
    }

    await new Promise<void>((resolve) => {
        const timeout = setTimeout(() => {
            resolve()
        }, 10)
    })
    
    return {
        supported,
        error,
        config,
        ...codec,
    }
}

const result = ref([])

codecs.forEach(async (c) => {
    const r = await probeDecoder(c)
    result.value.push(r)
})



</script>


<template>
    <div class="table">
        <div class="codec" v-for="(codec, index) in result" :key="index">
            <div>{{ codec.name }}</div>
            <div>{{ codec.kind }}</div>
            <div>OK: {{ codec.supported }}</div>
            <div>
                <div v-if="codec.error">Error: {{ codec.error }}</div>
            </div>
            <div>{{ codec.asc }}</div>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.table {
    font-size: 0.75rem;
    padding: 8px;

    >.codec {
        display: grid;
        grid-template-columns: 80px 80px 1fr 1fr 1fr;
        gap: 8px;
    }
}
</style>