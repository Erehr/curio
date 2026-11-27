<script setup>
// A tool's page: an optional side column, the body, and a footer with its buttons on the right.
</script>

<template>
  <div class="tool-page">
    <div class="tool-page-main">
      <aside v-if="$slots.side" class="tool-page-side"><slot name="side" /></aside>
      <main class="tool-page-body"><slot /></main>
    </div>
    <footer v-if="$slots.foot" class="tool-page-foot"><slot name="foot" /></footer>
  </div>
</template>
