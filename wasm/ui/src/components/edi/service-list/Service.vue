<script setup lang="ts">
import type * as Types from '@/types'

import HexValue from '@/components/ui/HexValue.vue'
import IconPlay from '@/components/icons/IconPlay.vue'
import IconStop from '@/components/icons/IconStop.vue'
import IconPause from '@/components/icons/IconPause.vue'
import { computed } from 'vue'

const props = defineProps<{ service: Types.Service }>()
defineEmits<{
  (event: 'select', sid: number): void
  (event: 'play', sid: number): void
  (event: 'stop'): void
}>()

const isPlaying = computed(() => {
  return props.service?.isPlaying
})

const hasDlPlus = computed(() => {
  return (props.service?.dl?.dl_plus ?? []).length
})
</script>

<template>
  <div @click.prevent="$emit('select', service.sid)" class="service">
    <div class="controls">
      <button v-if="!isPlaying" @click.prevent.stop="$emit('play', service.sid)">
        <IconPlay />
      </button>
      <button v-else @click.prevent.stop="$emit('stop')">
        <IconPause />
      </button>
    </div>
    <div class="info">
      <div class="svc">
        <span class="label">{{ service?.label ?? '-' }}</span>
        <small class="sid">
          <HexValue :value="service.sid" />
          <span> / {{ service.sid }}</span>
        </small>
      </div>
      <div class="dl">
        <span v-if="hasDlPlus" class="has-dl-plus-flag">DL+</span>
        <span v-if="service?.dl?.label" class="label">{{ service?.dl?.label }}</span>
      </div>
    </div>
    <div class="sls">
      <div class="container">
        <figure v-if="service.sls?.url">
          <img :src="service.sls.url" :alt="service.sls?.md5 ?? 'SLS'" />
        </figure>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.service {
  display: grid;
  grid-template-columns: 32px 1fr 72px;
  gap: 24px;
  padding: 8px;
  cursor: pointer;
  > .controls {
    display: flex;
    flex-direction: column;
    justify-content: center;
  }
  > .info {
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-width: 0;

    > .svc {
      display: flex;
      flex-grow: 1;
      > .label {
        flex-grow: 1;
        display: flex;
        align-items: center;
      }
    }

    > .dl {
      display: flex;
      flex: 1 1 auto;
      max-width: 100%;
      overflow: hidden;
      min-width: 0; /* allow text shrinking in flexbox */
      align-items: center;

      > .__has-dl-plus-flag {
        font-size: 0.75rem;
        margin-right: 4px;
        color: #fff;
        background: #000;
        padding: 2px;
      }

      > .has-dl-plus-flag {
        font-size: 0.75rem;
        margin-right: 6px;
        color: #000;
        background: #fff;
        padding: 2px 4px;
        border: 1px solid currentColor;
      }

      > .label {
        display: block;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        font-size: 0.75rem;
      }
    }
  }
  > .sls {
    > .container {
      background: hsl(var(--c-muted));
      width: 72px;
      height: 54px;
      aspect-ratio: 4/3;
      > figure {
        margin: 0;
        padding: 0;
        > img {
          max-width: 100%;
          object-fit: contain;
        }
      }
    }
  }
  /*
  > div {
    display: flex;
    flex-direction: column;
    justify-content: center;
  }
  */
}

button {
  width: 32px;
  height: 32px;
  cursor: pointer;
  margin: 0;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 0;
  background: transparent;
  > svg {
    width: inherit;
    height: inherit;
  }
}
</style>
