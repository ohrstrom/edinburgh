<script setup lang="ts">
import type * as Types from '@/types'

defineProps<{ subchannel: Types.Subchannel }>()

</script>

<template>
    <!--
    SCID: 17 - Start: 123 - CU: 16 - EEP 3-A
    -->
    <div class="subchannel">
        <span>SCID: {{ subchannel.id }}</span>
        <span>Start: {{ subchannel.start }}</span>
        <span>CU: {{ subchannel.size }}</span>
        <span>{{ subchannel.pl }}</span>
    </div>
</template>

<style lang="scss" scoped>
.subchannel {
    display: inline-flex;
    gap: 8px;
    > span {
        &::after {
            content: '•';
            margin-left: 8px;
        }
        &:last-child {
            &::after {
                content: '';
            }
        }
    }
}
</style>