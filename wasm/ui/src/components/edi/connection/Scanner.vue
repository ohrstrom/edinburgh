<script lang="ts" setup>

import { EDI } from '../../../../../pkg'

const props = defineProps<{ edi: EDI }>()

class EDIScanner {
    edi: EDI
    ws: WebSocket | null = null

    constructor({ edi }: { edi: EDI }) {
        this.edi = edi
    }


    async scan(): Promise<void> {
        console.log('Scanning...')
    }
}

const scanner = new EDIScanner({
    edi: props.edi,
})

</script>
<template>
    <div>
        <h2>(( SCANNER ))</h2>
        <div>TABLE</div>
        <div>
        </div>
    </div>
</template>