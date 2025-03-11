<template>
  <header class="app-header">
    <el-menu mode="horizontal" router :ellipsis="false">
      <el-menu-item index="/">
        多边形算法演示
      </el-menu-item>
      <el-menu-item index="/polygon-demo">
        点在多边形内算法
      </el-menu-item>
      <!-- 这里可以添加更多的demo菜单项 -->
    </el-menu>
  </header>
</template>

<style scoped>
.app-header {
  margin-bottom: 20px;
}
</style> 