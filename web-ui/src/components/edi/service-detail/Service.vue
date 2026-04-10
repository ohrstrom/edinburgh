<script setup lang="ts">
import { computed } from 'vue'

import type * as Types from '@/types'

import { storeToRefs } from 'pinia'
import { useEDIStore } from '@/stores/edi'

import HexValue from '@/components/ui/HexValue.vue'
import LevelMeter from '@/components/meter/LevelMeter.vue'
import DlPlusDisplay from './DlPlusDisplay.vue'
import IconLink from '@/components/icons/IconLink.vue'

import Subchannel from './Subchannel.vue'

const ediStore = useEDIStore()
const { selectedService: service } = storeToRefs(ediStore)

const af = computed(() => {
  if (!service.value) return null

  return service.value.audioFormat
})

const hasDlPlus = computed(() => {
  return (service.value?.dl?.dl_plus ?? []).length
})

defineProps<{ level: Types.Level }>()
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

          <span v-if="service?.language"> • {{ service.language }} </span>
        </div>
      </div>
      <div class="info-section format">
        <Subchannel v-if="service?.subchannel" :subchannel="service.subchannel" />
        <div v-if="af" class="audio-format">
          <span>{{ af.codec }}</span>
          <span>{{ af.samplerate }} kHz</span>
          <span>@ {{ af.bitrate }} kBit/s</span>
          <span v-if="af.channels == 2">Stereo</span>
          <span v-else>Mono</span>
          <span v-if="af.sbr || af.ps" class="flags">
            <span v-if="af.sbr">SBR</span>
            <span v-if="af.sbr && af.ps">+</span>
            <span v-if="af.ps">PS</span>
          </span>
        </div>
      </div>
      <div class="info-section dl-container" :class="{ 'has-dl-plus': hasDlPlus }">
        <div class="dl">
          <span v-if="service?.dl?.label" class="label">{{ service?.dl?.label }}</span>
        </div>
        <div v-if="hasDlPlus" class="dl-plus">
          <DlPlusDisplay
            v-for="l in service?.dl?.dl_plus"
            :key="l.kind"
            :kind="l.kind"
            :value="l.value"
            class="item"
          />
        </div>
      </div>
      <div class="info-section meter">
        <LevelMeter :level="level" />
      </div>
    </div>
    <div class="sls">
      <div class="container">
        <figure v-if="service.sls?.alternative_location_url" class="alternative-location-url">
          <img
            :src="service.sls.alternative_location_url"
            :alt="service.sls.alternative_location_url"
          />
          <!--
          <figcaption>
            <a :href="service.sls.alternative_location_url" target="_blank" class="url">{{ service.sls.alternative_location_url }}</a>
          </figcaption>
          -->
        </figure>
        <figure v-else-if="service.sls">
          <img :src="service.sls?.url" :alt="service.sls?.md5 ?? 'SLS'" />
          <figcaption>
            <span class="mimetype">{{ service.sls.mimetype }}</span>
            <span class="dimensions">{{ service.sls.width }}x{{ service.sls.height }} px</span>
            <span class="size">{{ ((service.sls?.len ?? 0) / 1000).toFixed(2) }} kB</span>
          </figcaption>
        </figure>
        <div v-if="service.sls?.click_through_url" class="click-through-url">
          <a
            :href="service.sls.click_through_url"
            :title="service.sls.click_through_url"
            target="_blank"
          >
            <IconLink :size="16" />
          </a>
        </div>
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
      margin-bottom: 4px;
      > .label {
        margin-bottom: 8px;
        font-size: 1.25rem;
      }
    }
    > .format {
      margin-bottom: 8px;
      font-family: var(--t-family-mono);
      .subchannel {
        font-size: var(--t-fs-s);
      }
      .audio-format {
        display: flex;
        gap: 8px;
        font-size: var(--t-fs-s);

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

        > .has-dl-plus-flag {
          font-size: var(--t-fs-s);
          margin-right: 6px;
          color: hsl(var(--c-fg));
          background: hsl(var(--c-bg));
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
          grid-template-columns: 90px 1fr;
          font-family: var(--t-family-mono);
          font-size: var(--t-fs-s);
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
      background: hsl(var(--c-fg) / 0.05);
      width: 324px;
      height: 244px;
      aspect-ratio: 4/3;
      display: flex;
      align-items: center;
      justify-content: center;
      position: relative;
      > figure {
        display: flex;
        align-items: center;
        justify-content: center;
        margin: 0;
        padding: 0;
        width: 320px;
        height: 240px;
        background: #ff00ff; /* mark bad ratio */
        > img {
          max-width: 100%;
          object-fit: contain;
        }
        > figcaption {
          color: hsl(var(--c-bg));
          background: hsl(var(--c-fg) / 75%);
          font-family: var(--t-family-mono);
          font-size: var(--t-fs-s);
          position: absolute;
          bottom: 2px;
          left: 2px;
          right: 2px;
          padding: 2px 4px;
          display: flex;
          justify-content: space-between;
          opacity: 0;

          transition: opacity 100ms ease-in-out;

          > .mimetype {
            text-transform: uppercase;
          }

          > .url {
            padding: 2px 4px;
            font-size: var(--t-fs-xs);
            display: inline-block;
            max-width: 100%;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
          }
        }
      }

      &:hover {
        > figure {
          > figcaption {
            opacity: 1;
          }
        }
      }

      > .click-through-url {
        position: absolute;
        top: 6px;
        right: 6px;
        display: flex;
        align-items: center;
        justify-content: center;

        > a {
          width: 24px;
          height: 24px;
          background: hsl(var(--c-fg) / 75%);
          border-radius: 2px;
          display: flex;
          align-items: center;
          justify-content: center;
          color: hsl(var(--c-bg));

          &:hover {
            color: hsl(var(--c-cta-fg));
            background: hsl(var(--c-cta));
          }
        }
      }
    }
  }
  &--skeleton {
    min-height: 180px;
    font-family: var(--t-family-mono);
    > .info {
      .message {
        display: inline-flex;
        padding: 2px 4px;
        font-size: var(--t-fs-s);
      }
    }
  }
}
</style>
