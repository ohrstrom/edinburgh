<script setup lang="ts">

import HexValue from "@/components/ui/HexValue.vue";
import IconPlay from "@/components/icons/IconPlay.vue";
import IconStop from "@/components/icons/IconStop.vue";

interface Service {
  sid: number
  scid?: number
  label?: string
  short_label?: string
  isCurrent: boolean
}

defineProps<{ service: Service }>()
defineEmits<{
  // (event: 'select', payload: { sid: number }): void
  // (event: 'play', payload: { scid?: number }): void
  (event: 'select', payload: { sid: number, scid: number }): void
  (event: 'play', payload: { scid?: number }): void
}>()
</script>

<template>
  <div @click.prevent="$emit('select', { sid: service.sid, scid: service.scid })" class="service">
    <div class="controls">
      <button @click.prevent="$emit('play', { scid: service.scid })">
        <IconPlay v-if="!service.isCurrent" />
        <IconStop v-else />
      </button>
      <!--
      <button @click="$emit('play', { scid: service.scid })">
        {{ service.isCurrent ? 'S' : 'P' }}
      </button>
      -->
    </div>
    <div class="info">
      <div class="svc">
        <span class="label">{{ service?.label ?? '-' }}</span>
        <small class="sid">
          <HexValue :value="service.sid" />
<!--          <span> / {{ service.sid }}</span>-->
        </small>
      </div>
      <div class="dl">
        <small v-if="service.dl">{{ service.dl.label }}</small>
        <small v-else>&nbsp;</small>
      </div>
    </div>
    <div class="sls">
      <div class="container">
        <figure v-if="service.sls?.url">
          <img :src="service.sls.url" />
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
      flex: 1 1 auto;
      max-width: 100%;
      overflow: hidden;
      min-width: 0; /* 💥 important to allow text shrinking in flexbox */

      > small {
        display: block;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }
    }
  }
  > .sls {
    > .container {
      background: #efefef;
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
