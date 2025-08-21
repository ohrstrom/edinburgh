<script setup lang="ts">
import type * as Types from '@/types'

import HexValue from '@/components/ui/HexValue.vue'
import IconPlay from '@/components/icons/IconPlay.vue'
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
      <button class="play" v-if="!isPlaying" @click.prevent.stop="$emit('play', service.sid)">
        <IconPlay />
      </button>
      <button class="pause" v-else @click.prevent.stop="$emit('stop')">
        <IconPause />
      </button>
    </div>
    <div class="svc">
      <div class="label">
        <span>{{ service?.label ?? '-' }}</span>
      </div>
      <div class="dl">
        <span v-if="hasDlPlus" class="has-dl-plus-flag">DL+</span>
        <span v-if="service?.dl?.label" class="label">{{ service?.dl?.label }}</span>
      </div>
      <div class="details">
        <HexValue class="sid" :value="service.sid" />
        <span v-text="service.audioFormat?.codec ?? '-'" />
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

    > button {
      background: hsl(var(--c-fg) / 0.05);
      border-radius: 50%;

      &.play {
        &:hover {
          background: hsl(var(--c-cta));
        }
      }

      &.pause {
        background: hsl(var(--c-cta) / 0.05);
        &:hover {
          background: hsl(var(--c-cta));
        }
      }
    }
  }
  > .svc {
    display: grid;
    min-width: 0;
    grid-template-areas:
      'label details'
      'dl    details';

    > .label {
      grid-area: label;
      display: flex;
      align-items: center;
    }

    > .dl {
      grid-area: dl;
      display: flex;
      flex: 1 1 auto;
      max-width: 100%;
      overflow: hidden;
      min-width: 0; /* allow text shrinking in flexbox */
      align-items: flex-start;

      > .has-dl-plus-flag {
        font-size: var(--t-fs-xs);
        font-family: var(--t-family-mono);
        margin-right: 6px;
        // color: hsl(var(--c-bg));
        // background: hsl(var(--c-fg));
        background: hsl(var(--c-mark));
        color: hsl(var(--c-mark-fg));
        padding: 2px 4px;
        border-radius: var(--b-r-s);
      }

      > .label {
        display: block;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        font-size: var(--t-fs-s);
      }
    }

    > .details {
      grid-area: details;
      font-size: var(--t-fs-s);
      display: flex;
      align-items: flex-end;
      flex-direction: column;
    }
  }
  > .sls {
    > .container {
      background: hsl(var(--c-fg) / 0.05);
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
