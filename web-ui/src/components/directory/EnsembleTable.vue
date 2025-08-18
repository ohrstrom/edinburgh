<script lang="ts" setup>
import { ref, onMounted, computed } from 'vue'

import HexValue from '@/components/ui/HexValue.vue'

const directoryUrl = ref<string>('http://localhost:9001/ensembles')

defineEmits<{
  (event: 'select', payload: { host: string; port: number }): void
}>()

onMounted(async () => {
  try {
    errors.value = []
    const response = await fetch(directoryUrl.value)
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`)
    }
    const data = await response.json()
    ensembleList.value = data
  } catch (error) {
    console.error('Error fetching ensemble directory:', error)
    errors.value.push(error)
  }
})

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const errors = ref<any[]>([])
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const ensembleList = ref<any[]>([])

const ensembleListSorted = computed(() => {
  return [...ensembleList.value].sort((a, b) => {
    const hostC = a.host.localeCompare(b.host)
    if (hostC !== 0) {
      return hostC
    }
    return a.label.localeCompare(b.label)
  })
})

const ensembles = computed(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return ensembleListSorted.value.map((ensemble: any) => {
    const cus = ensemble.subchannels.reduce((t: number, c: number) => t + c.size, 0)
    return {
      ...ensemble,
      cus,
      services: ensemble.services || [],
    }
  })
})
</script>

<template>
  <div class="ensemble-table">
    <div v-if="ensembles.length" class="table">
      <div
        class="ensemble"
        v-for="(ensemble, index) in ensembles"
        :key="`table-ensemble-${index}`"
        @click.prevent="$emit('select', { host: ensemble.host, port: ensemble.port })"
      >
        <HexValue class="eid" :value="ensemble.eid" />
        <span class="label">{{ ensemble?.label ?? '-' }}</span>
        <span class="host">{{ ensemble.host }}:{{ ensemble.port }}</span>
        <span class="cus">
          <span>{{ ensemble.cus }} CUs</span>
        </span>
        <span class="services">
          <span>{{ (ensemble?.services ?? []).length }} SVCs</span>
        </span>
      </div>
    </div>
    <div v-else class="table table--skeleton">
      <div class="info">
        <span>no ensembles scanned</span>
        <p>{{ directoryUrl }}</p>
        <pre v-if="errors.length" class="errors" v-text="errors" />
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.ensemble-table {
  border-top: 1px solid hsl(var(--c-fg));
  font-family: var(--t-family-mono);
}
.table {
  font-size: var(--t-fs-s);
  padding: 8px;
  overflow-y: auto;
  max-height: 25vh;
  cursor: pointer;

  /* scrollbar */
  &::-webkit-scrollbar {
    width: 4px;
    background: hsl(var(--c-fg) / 0.1);
  }

  &::-webkit-scrollbar-thumb {
    background: hsl(var(--c-fg));
    border-radius: 0;
  }

  > .ensemble {
    display: grid;
    grid-template-columns: 80px 2fr 1fr 80px 80px;
    gap: 8px;
    padding: 2px 8px;

    &:hover {
      background: hsl(var(--c-fg) / 0.05);
    }

    > .eid {
      text-align: end;
    }

    > .services {
      text-align: end;
      display: inline-flex;
      gap: 4px;
    }
  }
  &--skeleton {
    padding: 8px;
    > .info {
      .message {
        display: inline-flex;
        color: hsl(var(--c-fg));
        padding: 2px 4px;
        font-size: var(--t-fs-s);
      }
    }
  }
}
</style>
