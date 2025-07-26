<template>
  <div class="panel">
    <div class="panel-header">
      <slot name="header" />
    </div>
    <div class="panel-default">
      <slot />
    </div>
    <div class="panel-sub">
      <div class="navigation">
        <slot name="sub-navigation" />
      </div>
      <div class="content">
        <slot name="sub-content" />
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.panel {
  border: 1px solid hsl(var(--c-fg));
  box-shadow: 4px 4px hsl(var(--c-shadow));
  background: hsl(var(--c-bg));
  border-radius: var(--b-r-s);
  .panel-header {
    grid-column: 1 / -1;
    margin-left: -8px;
    margin-right: -8px;
  }
  .panel-default {
    display: contents;
  }
  .panel-sub {
    grid-column: 1 / -1;
    margin-left: -8px;
    margin-right: -8px;

    > .navigation {
      display: contents;
    }

    > .content {
      display: contents;
    }
  }
  .panel-sub-navigation {
    grid-column: 1 / -1;
    margin-left: -8px;
    margin-right: -8px;
  }
  .panel-sub-content {
    grid-column: 1 / -1;
    margin-left: -8px;
    margin-right: -8px;
  }
}
</style>
