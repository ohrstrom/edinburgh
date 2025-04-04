<template>
  <div class="panel">
    <div class="panel-header">
      <slot name="header" />
    </div>
    <div class="panel-content">
      <slot />
    </div>
    <div class="panel-header">
      <slot name="footer" />
    </div>
  </div>
</template>

<style lang="scss" scoped>
.panel {
  border: 1px solid #000;
  box-shadow: 4px 4px #000;
  background: hsl(var(--c-bg));
  .panel-header {
    grid-column: 1 / -1;
    margin-left: -8px;
    margin-right: -8px;
  }
  .panel-content {
    display: contents;
  }
  .panel-footer {
    grid-column: 1 / -1;
    margin-left: -8px;
    margin-right: -8px;
  }
}
</style>
