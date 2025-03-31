<script setup lang="ts">

import {storeToRefs} from "pinia";
import {useEDIStore} from "@/stores/edi";

const ediStore = useEDIStore()

const {selectedSid} = storeToRefs(ediStore)

</script>

<template>
  <div class="service">
    (( SERVICE ))
    {{ selectedSid }}
  </div>
</template>