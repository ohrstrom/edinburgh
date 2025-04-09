<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useStorage } from '@vueuse/core'
import { useEDIStore } from '@/stores/edi'

import HexValue from '@/components/ui/HexValue.vue'

const ediStore = useEDIStore()
const { ensemble, audioFormats } = storeToRefs(ediStore)

defineEmits<{
  (event: 'select', sid: number): void
}>()

const expanded = useStorage('edi/ensemble/service-table/expanded', false)

const services = computed(() => {
  if (!ensemble.value) return []

  return ensemble.value.services.flatMap((svc) => {
    return svc.components.map((comp) => {
      const subchannel = ensemble.value.subchannels.find((sc) => sc.id === comp.subchannel_id)

      return {
        sid: svc.sid,
        label: svc.label,
        short_label: svc.short_label,
        scid: comp.scid,
        language: comp.language,
        user_apps: comp.user_apps,
        audioFormat: audioFormats.value.get(comp.scid),
        subchannel,
      }
    })
  }).sort((a, b) => a.scid - b.scid)
})

</script>

<template>
  <div class="ensemble-table">
   <div class="header">
    <div @click.prevent="expanded = !expanded" class="toggle">
      <span class="label">Service Table</span>
      <span v-if="expanded" class="icon icon--close">⌃</span>
      <span v-else class="icon icon--open">⌄</span>
    </div>
   </div>
    
    <div v-if="expanded && services.length" class="table">
      <div class="service" v-for="(svc, index) in services ?? []" :key="`table-svc-${index}`" @click.prevent="$emit('select', svc.sid)">
        <span class="scid">{{ svc.scid }}</span>
        <HexValue class="sid" :value="svc.sid" />
        <span class="label">{{ svc?.label ?? '-' }}</span>
        <span class="short-label">{{ svc?.short_label ?? '-' }}</span>
        <span class="language">{{ svc?.language ?? '-' }}</span>
        <span class="user-apps">
          <span v-if="svc?.user_apps">
          {{ svc.user_apps.join(', ') }}
          </span>
        </span>
        <span class="audio-format">
          <span v-if="svc?.audioFormat">
            <span class="codec">{{ svc.audioFormat.codec }}</span>
            <span>{{ svc.audioFormat.samplerate }} kHz</span>
            <span>@ {{ svc.audioFormat.bitrate }} kBit/s</span>
            <span v-if="svc.audioFormat.channels == 2">S</span>
            <span v-else>M</span>
          </span>
        </span>
        <span class="subchannel">
          <span v-if="svc?.subchannel">
            SA: {{ String(svc.subchannel.start).padStart(3, '0') }}
            CU: {{ svc.subchannel.size }}
            • {{ svc.subchannel.pl }}
          </span>
        </span>
      </div>
    </div>
    <!-- 
    <pre v-text="ensemble" />
    -->
  </div>
</template>

<style lang="scss" scoped>
.ensemble-table {
  /* border-top: 1px solid #000; */
}

.header {
  border-top: 1px solid #000;
  border-bottom: 1px solid #000;
  padding-left: 8px;
  > .toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 24px;
    cursor: pointer;

    > .label {
      font-size: 0.75rem;
    }

    > .icon {
      &--open {
        margin-top: -9px;
      }
      &--close {
        margin-top: 5px;
      }
    }
  }
}

.table {
  padding-top: 8px;
  padding-bottom: 8px;;
  font-size: 0.75rem;
  .service {
    display: grid;
    grid-template-columns: 32px 48px 2fr 1fr 1fr 48px 3fr 3fr;
    gap: 8px;
    padding: 2px 8px;
    cursor: pointer;

    strong {
      font-weight: 600;
    }

    &:hover {
      background: hsl(var(--c-muted));
    }

    > .scid {
      text-align: end;
    }

    > .audio-format {
      > span {
        display: flex;
        gap: 4px;
        > .codec {
          min-width: 60px;
        }
      }
    }
  }
}
</style>