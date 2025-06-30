<script lang="ts" setup>
import {ref, Ref} from 'vue'

import { EDI } from '../../../../../pkg'

import type * as Types from '@/types'

const props = defineProps<{ edi: EDI }>()

class EDIScanner {
    edi: EDI
    ws: WebSocket | null = null
    ensemble: Types.Ensemble | null = null

    // store
    // ensembles: Ref<Types.Ensemble[]> = ref([])
    ensembles: Ref<Map<number, Types.Ensemble>> = ref(new Map())

    constructor({ edi }: { edi: EDI }) {
        this.edi = edi

        edi.addEventListener("ensemble_updated", async (e: CustomEvent) => {
            await this.updateEnsemble(e.detail as Types.Ensemble)
        })
    }

    async updateEnsemble(ensemble: Types.Ensemble): Promise<void> {
        console.log('SCANNER: Ensemble updated:', ensemble)
        this.ensemble = ensemble
        return Promise.resolve()
    }

    async scanPort (port: number): Promise<void> {
        console.log('SCANNER: Scan port:', port)
        await this.edi.reset()
        this.ws = null
        this.ensemble = null

        const host = 'edi-ch.digris.net'
        const uri = `ws://localhost:9000/ws/${host}/${port}/`

        const ws = new WebSocket(uri)
        ws.binaryType = 'arraybuffer'


        ws.onmessage = (event) => {
          this.edi.feed(new Uint8Array(event.data))
        }

        // wait for ensemble.eid or timeout
        await new Promise<void>((resolve) => {
            const checkInterval = 50  // ms
            let interval: number
            const timeout = setTimeout(() => {
                console.log('SCANNER: Timeout')
                clearInterval(interval)
                ws.close()
                resolve()
            }, 1000)

            interval = setInterval(() => {
                if (this.ensemble?.label) {
                    console.log('SCANNER: Got valid ensemble:', this.ensemble)

                    this.ensembles.value.set(port, this.ensemble)

                    clearTimeout(timeout)
                    clearInterval(interval)
                    ws.close()
                    resolve()
                }
            }, checkInterval)
        })
    }

    async scan(): Promise<void> {
        console.log('SCANNER: Start scan')

        const ports = Array(30).fill(8850).map((x, y) => x + y)

        for (const port of ports) {
            await this.scanPort(port)
        }
    }
}

const scanner = new EDIScanner({
    edi: props.edi,
})

</script>

<template>
    <div class="scanner">
        <div class="table">
            <div class="ensemble" v-for="[port, ensemble] in scanner.ensembles.value.entries()" :key="`scanner-ensemble-port-${port}`">
                <div v-text="port" />
                <div v-text="ensemble.eid" />
                <div v-text="ensemble.label" />
            </div>
        </div>
        <div>
            <button @click="scanner.scan()">Scan</button>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.table {
    padding: 8px 0;
    font-size: 0.75rem;
    > .ensemble {
        display: grid;
        grid-gap: 8px;
        grid-template-columns: auto auto 1fr;
    }
}
</style>