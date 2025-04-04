<script setup lang="ts">
import { computed } from 'vue'

import type * as Types from '@/types'

import { storeToRefs } from 'pinia'
import { useEDIStore } from '@/stores/edi'

import HexValue from '@/components/ui/HexValue.vue'
import LevelMeter from '@/components/meter/LevelMeter.vue'

import Subchannel from './Subchannel.vue'

const ediStore = useEDIStore()
const { selectedService: service } = storeToRefs(ediStore)

const af = { is_sbr: true, is_ps: true, codec: 'HE-AAC', samplerate: 48, bitrate: 48, au_count: 3 }

const hasDlPlus = computed(() => {
  return (service.value?.dl?.dl_plus ?? []).length
})

defineProps<{ level: Types.Level }>()

// const dummyLabel =  "ARTBAT - Love is Gonna Save And Some More Text we should scroll Us (with Benny Benassi) - radio4tng.ch"
</script>

<template>
  <div v-if="service" class="service">
    <!--
    <pre v-text="{service}" />
    -->
    <div class="info">
      <div class="info-section svc">
        <h2 class="label">{{ service?.label ?? '-' }}</h2>
        <div>
          <span>{{ service?.short_label ?? '-' }}</span>
          <span v-if="service?.short_label">&nbsp;•&nbsp;</span>
          <HexValue :value="service.sid" />

          <span v-if="service?.language">
            • {{ service.language }}
          </span>
        </div>
      </div>
      <div class="info-section format">
        <Subchannel v-if="service?.subchannel" :subchannel="service.subchannel" />
        <div v-if="af" class="audio-format">
          <span>{{ af.codec }}</span>
          <span>{{ af.samplerate }} kHz</span>
          <span>@ {{ af.bitrate }} kBit/s</span>
          <span>Stereo</span>
          <span v-if="af.is_sbr || af.is_ps" class="flags">
            <span v-if="af.is_sbr">SBR</span>
            <span v-if="af.is_sbr && af.is_ps">+</span>
            <span v-if="af.is_ps">PS</span>
          </span>
        </div>
      </div>
      <div class="info-section dl-container" :class="{ 'has-dl-plus': hasDlPlus }">
        <div class="dl">
          <span v-if="hasDlPlus" class="has-dl-plus-flag">DL+</span>
          <span v-if="service?.dl?.label" class="label">{{ service?.dl?.label }}</span>
          <!--
          <span class="label">{{ dummyLabel }}</span>
          -->
        </div>
        <div v-if="hasDlPlus" class="dl-plus">
          <div v-for="l in service?.dl?.dl_plus" :key="l.kind" class="item">
            <span class="kind" v-text="l.kind.replace('_', '.')" />
            <span class="value" v-text="l.value" />
          </div>
        </div>
      </div>
      <div class="info-section meter">
        <LevelMeter :level="level" />
      </div>
    </div>
    <div class="sls">
      <div class="container">
        <figure v-if="service.sls?.url">
          <img :src="service.sls?.url" :alt="service.sls?.md5 ?? 'SLS'" />
        </figure>
      </div>
    </div>
  </div>
  <div v-else class="service service--skeleton">
    <div class="info">
      <span class="message">no service selected</span>
    </div>
    <div class="sls">
      <div class="container" />
    </div>
  </div>
</template>

<style lang="scss" scoped>
.service {
  display: grid;
  grid-template-columns: 1fr 324px;
  grid-gap: 24px;
  > .info {
    display: flex;
    flex-direction: column;
    min-width: 0;
    .svc {
      margin-bottom: 16px;
      > .label {
        margin-bottom: 8px;
        font-size: 1.25rem;
      }
    }
    > .format {
      .subchannel {
        font-size: 0.75rem;
      }
      .audio-format {
        display: flex;
        gap: 8px;
        font-size: 0.75rem;

        .flags {
          &::before {
            content: '(';
          }
          &::after {
            content: ')';
          }
          > span {
            display: contents;
          }
        }
      }
    }

    .dl-container {
      display: flex;
      flex-direction: column;
      flex-grow: 1;
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
        }
      }
      > .dl-plus {
        display: flex;
        flex-direction: column;
        gap: 4px;
        padding: 8px 0 16px;
        > .item {
          display: grid;
          grid-template-columns: 100px 1fr;
          font-size: 0.75rem;
          //line-height: 0.75rem;
          > .kind {
            //color: #666;
            &::after {
              content: ':';
            }
          }
          > .value {
            font-style: inherit;
          }
        }
      }
    }

    > .meter {
      margin-top: auto;
    }
  }
  > .sls {
    > .container {
      background: hsl(var(--c-muted));
      width: 324px;
      height: 244px;
      aspect-ratio: 4/3;
      display: flex;
      align-items: center;
      justify-content: center;
      > figure {
        margin: 0;
        padding: 0;
        width: 320px;
        height: 240px;
        > img {
          max-width: 100%;
          object-fit: contain;
        }
      }
    }
  }
  &--skeleton {
    min-height: 180px;
    > .info {
      .message {
        display: inline-flex;
        color: black;
        padding: 2px 4px;
        font-size: 0.75rem;
      }
    }
  }
}
</style>
