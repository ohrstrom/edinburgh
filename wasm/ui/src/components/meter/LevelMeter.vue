<script setup lang="ts">
import { computed, useTemplateRef, reactive, watchEffect, onMounted, onUnmounted } from "vue";
import { useElementSize } from '@vueuse/core'

import type * as Types from '@/types'

const props = defineProps<{ volume: Types.Volume }>();

const scale = [
    -48,
    -44,
    -40,
    -36,
    -32,
    -28,
    -24,
    -20,
    -16,
    -12,
    -8,
    -4,
    0,
]

const levelsEl = useTemplateRef('levelsEl')
const { width: levelsWidth } = useElementSize(levelsEl)

const tickWidth = computed(() => {
  return levelsWidth.value / (scale.length - 1)
})

const dBFS = computed(() => {
  return {
    l: 20 * Math.log10(Math.max(props.volume.l, 0.00001)),
    r: 20 * Math.log10(Math.max(props.volume.r, 0.00001)),
  }
})

const levelWidth = computed(() => {
  const min = -48
  const max = 0

  const clamp = (val: number) => Math.max(min, Math.min(max, val))

  const percent = (val: number) => ((clamp(val) - min) / (max - min)) * 100

  return {
    l: percent(dBFS.value.l),
    r: percent(dBFS.value.r),
  }
})

let rafId: number
const levelWidthDecay = reactive({ l: 0, r: 0 }) // displayed % values

onMounted(() => {
  const decayPerSecond = 70
  let lastFrame = performance.now()

  const update = (now = performance.now()) => {
    const delta = (now - lastFrame) / 1000
    lastFrame = now

    const fallSpeed = decayPerSecond * delta

    if (levelWidth.value.l > levelWidthDecay.l) {
      levelWidthDecay.l = levelWidth.value.l
    } else {
      levelWidthDecay.l = Math.max(levelWidthDecay.l - fallSpeed, levelWidth.value.l)
    }

    if (levelWidth.value.r > levelWidthDecay.r) {
      levelWidthDecay.r = levelWidth.value.r
    } else {
      levelWidthDecay.r = Math.max(levelWidthDecay.r - fallSpeed, levelWidth.value.r)
    }

    rafId = requestAnimationFrame(update)
  }

  update()
})

onUnmounted(() => {
  cancelAnimationFrame(rafId)
})


</script>

<template>
  <!--
  <pre v-text="{dBFS, levelWidthDecay}" />
  -->

  <div class="meter">
    <div class="legend">
      <div class="channel">
        <span>L</span>
      </div>
      <div class="unit">
        <span>dbFS</span>
      </div>
      <div class="channel">
        <span>R</span>
      </div>
    </div>
    <div class="levels" ref="levelsEl">
      <div class="bar">
        <div class="level" :style="{ width: levelWidthDecay.l + '%' }" />
      </div>
      <div class="scale">
        <div v-for="(dB, i) in scale" :key="i" class="tick" :style="{ left: tickWidth * i - 14 + 'px' }">
          <span v-text="dB" class="tick-label" />
        </div>
      </div>
      <div class="bar">
        <div class="level" :style="{ width: levelWidthDecay.r + '%' }" />
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.meter {
  display: grid;
  grid-template-columns: 60px 1fr;

  background: white;

  .legend {
    background: #efefef;
    //color: #fff;
    font-size: 0.75rem;
    line-height: 12px;
    .unit {
      height: 22px;
      display: flex;
      align-items: center;
      //justify-content: center;
      padding-left: 4px;
      padding-right: 4px;
    }
    .channel {
      height: 18px;
      display: flex;
      align-items: center;
      //justify-content: center;
      padding-left: 4px;
      padding-right: 4px;
    }
  }

  .levels {
    .scale {
      position: relative;
      display: grid;
      grid-template-columns: repeat(13, 1fr);
      height: 22px;
      font-size: 0.75rem;
      line-height: 12px;
      align-items: center;
      .tick {
        color: #000;
        position: absolute;
        height: 22px;
        width: 28px;
        top: 0;
        left: 0;
        display: flex;
        align-items: center;
        justify-content: center;

        .tick-label {
          margin-left: -8px;
        }

        &::before {
          content: '';
          position: absolute;
          top: -4px;
          left: 14px;
          width: 1px;
          height: 8px;
          background: currentColor;
        }
        &::after {
          content: '';
          position: absolute;
          bottom: -4px;
          left: 14px;
          width: 1px;
          height: 8px;
          background: currentColor;
        }

        &:first-child {
          &::before,
          &::after {
            display: none;
          }
        }

        &:last-child {
          .tick-label {
            margin-left: -2px;
          }
          &::before,
          &::after {
            left: 13px;
          }
        }

      }
    }
    .bar {
      height: 18px;
      background: #efefef;
      border-left: 1px solid currentColor;
      .level {
        height: 18px;
        width: 20%;
        background: #2cf54e;
      }
    }
  }
}
</style>
