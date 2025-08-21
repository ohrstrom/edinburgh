<script setup lang="ts">
import pkg from '../../../package.json'

const version = pkg?.version ?? '-'
const homepage = pkg?.homepage
</script>

<template>
  <footer>
    <div class="version">
      <div class="item">
        <span class="key">Version</span>
        <span class="value">{{ version }}</span>
      </div>
      <!--
            <div class="item">
                <span class="key">REF</span>
                <span class="value">a35e8fc</span>
            </div>
            -->
    </div>
    <div class="links">
      <a v-if="homepage" :href="homepage" target="_blank">GitHub</a>
    </div>
  </footer>
</template>

<style lang="scss" scoped>
footer {
  font-size: var(--t-fs-s);
  font-family: var(--t-family-mono);
  color: hsl(var(--c-fg) / 0.75);

  display: flex;
  justify-content: space-between;

  padding: 0 0.25rem;

  > .version {
    display: flex;
    gap: 1rem;
    justify-content: center;

    > .item {
      > .key {
        margin-right: 0.25rem;
        &::after {
          content: ':';
        }
      }

      > .value {
        font-style: normal;
      }
    }
  }

  > .links {
    display: flex;
    gap: 1rem;
    > a {
      text-decoration: underline;
    }
  }
}
</style>
