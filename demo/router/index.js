import { createRouter, createWebHistory } from 'vue-router'
import Home from '../views/Home.vue'
import PolygonDemo from '../views/PolygonDemo.vue'

const routes = [
  {
    path: '/',
    name: 'Home',
    component: Home
  },
  {
    path: '/polygon-demo',
    name: 'PolygonDemo',
    component: PolygonDemo
  }
  // 这里可以添加更多的demo路由
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

export default router 