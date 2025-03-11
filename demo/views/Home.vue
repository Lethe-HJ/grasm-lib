<template>
  <div class="home-container">
    <el-card>
      <template #header>
        <div class="card-header">
          <h1>多边形算法演示</h1>
        </div>
      </template>
      <div class="demo-list">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-card shadow="hover" @click="$router.push('/polygon-demo')">
              <h2>点在多边形内算法 - 扫描线可视化</h2>
              <p>展示了如何使用扫描线算法判断点是否在多边形内部</p>
              <el-button type="primary">查看演示</el-button>
            </el-card>
          </el-col>
          <!-- 这里可以添加更多的demo卡片 -->
        </el-row>
      </div>
    </el-card>
  </div>
</template>

<style scoped>
.home-container {
  width: 100%;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.demo-list {
  margin-top: 20px;
}
.el-card {
  margin-bottom: 20px;
  cursor: pointer;
}
.el-button {
  margin-top: 15px;
}
</style> 