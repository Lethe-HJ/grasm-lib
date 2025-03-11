<template>
  <div class="app-container">
    <AppHeader />
    <main>
      <router-view />
    </main>
  </div>
</template>

<script setup>
import AppHeader from './components/AppHeader.vue'
</script>

<style>
body {
  margin: 0;
  padding: 0;
  font-family: var(--el-font-family);
}
.app-container {
  width: 100%;
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px;
}
</style> 