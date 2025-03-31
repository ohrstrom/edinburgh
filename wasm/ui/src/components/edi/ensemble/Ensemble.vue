<script setup lang="ts">

import {storeToRefs} from "pinia";
import {useEDIStore} from "@/stores/edi";

import HexValue from "@/components/ui/HexValue.vue";

const ediStore = useEDIStore()
const { ensemble } = storeToRefs(ediStore)
</script>

<template>
  <div class="ensemble">
    <!--
    <pre v-text="{ensemble}" />
    -->
    <div>
      <h2>{{ ensemble?.label ?? '-' }}</h2>
      <small>{{ ensemble?.short_label ?? '-' }}</small>
    </div>
    <div>
      <span>EID: <HexValue :value="ensemble.eid" /></span>
    </div>
    <div>
      <span>SVC: {{ ensemble.services.length }}</span>
    </div>
  </div>
</template>
