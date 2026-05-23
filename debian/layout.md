# deb 打包目录布局

```
debian/
├── control              # 包元数据
├── postinst             # 安装后脚本（注册 systemd service）
├── wf-engine.service    # systemd user service 文件
└── layout.md            # 本文件

打包后目录结构:
/opt/workflow-engine/
├── wf-cli               # 独立 HTTP server 二进制 (ARM64, 静态链接)
├── dist/                # Vue 前端 build 产物
│   ├── index.html
│   └── assets/
├── wf-engine.service    # systemd service 副本（运行时用）
└── data/                # 运行时数据（由 postinst 创建）
    ├── logs/            # 每日轮转日志
    └── engine.db        # SQLite 数据库（自动创建）
```

## 构建 deb

```bash
# 1. 编译 ARM64 二进制
#    方式 A: GitHub Actions ubuntu-24.04-arm 原生编译
#    方式 B: WSL Linaro 交叉编译

# 2. 构建前端
cd frontend && npm run build

# 3. 打包 deb
mkdir -p pkg/opt/workflow-engine pkg/usr/lib/systemd/user
cp target/aarch64-unknown-linux-gnu/release/wf-cli pkg/opt/workflow-engine/
cp -r dist pkg/opt/workflow-engine/
cp debian/wf-engine.service pkg/opt/workflow-engine/
cp debian/wf-engine.service pkg/usr/lib/systemd/user/

# 4. 构建 .deb (需要 dpkg-deb，在 Docker uos:arm64 中执行)
dpkg-deb --build pkg workflow-engine_7.1.0_arm64.deb
```
