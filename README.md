### http-reverse-proxy

一个简单的反向代理工具，支持`HTTP`、`HTTPS`两种协议。

### 依赖
- [tokio](https://github.com/tokio-rs/tokio)
- [hyper](https://github.com/hyperium/hyper)
- [tower](https://github.com/tower-rs/tower)

### 配置文件

```yaml
# 服务配置
server:
  # 服务监听端口
  port: 8000

# 客户端配置
client:
  # 每个Host连接池最大空闲数
  pool_max_idle_per_host: 10
  # 连接空闲超时时间(超过时间将被关掉)
  max_idle_timeout: 60s

# 日志配置
logging:
  # 日志级别配置
  level:
    # 默认级别
    root: debug

# 路由配置
routes:
  # 百度服务路由 http://ip:port/baidu/index.html -> https://www.baidu.com/index.html 
  - id: baidu
    # 实际访问服务地址
    uri: https://www.baidu.com
    # 拦截规则
    predicate: /baidu/*
    # 路径strip级别 (这里会截取掉/baidu，默认为0，不截取)
    strip: 1
  - id: backend
    uri: http://127.0.0.1:7000
    predicate: /backend/*
```
