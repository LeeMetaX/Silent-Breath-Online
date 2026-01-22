# 🎥 1D Streaming Methods - i9-12900K Kernel Live View

## ✅ Active Services (Running Now)

All services are LIVE and streaming your kernel boot on private IP: **21.0.0.152**

| Service | Port | Status | URL |
|---------|------|--------|-----|
| MJPEG Stream | 8080 | ✅ LIVE | http://21.0.0.152:8080/stream.mjpg |
| Stream Viewer | 8080 | ✅ LIVE | http://21.0.0.152:8080/ |
| noVNC Console | 6080 | ✅ LIVE | http://21.0.0.152:6080/vnc.html |
| VNC Server | 5900 | ✅ LIVE | vnc://21.0.0.152:5900 |

---

## 🌐 Method 1: Direct Browser Access (If You Can Route to 21.0.0.152)

Simply open in your browser:
```
http://21.0.0.152:8080/
```

This gives you a beautiful live stream viewer with auto-refresh.

**Direct stream URL** (for embedding):
```
http://21.0.0.152:8080/stream.mjpg
```

---

## 🚀 Method 2: Deploy to Vercel (Recommended for Public Access)

### Quick Deploy:
```bash
# Download the Vercel project
cd /tmp/vercel-viewer

# Deploy to Vercel
vercel --prod
```

### What you get:
- Public URL (e.g., `https://your-kernel-viewer.vercel.app`)
- Automatic HTTPS
- Global CDN distribution
- Beautiful viewer interface

**Files included**:
- `index.html` - Full-featured stream viewer
- `vercel.json` - Deployment config
- `README.md` - Complete documentation

**Package location**: `/tmp/kernel-viewer.tar.gz` (3.7KB)

---

## 🔌 Method 3: SSH Tunnel (For Private Access)

If you have SSH access to the sandbox:

```bash
# Tunnel the MJPEG stream
ssh -L 8080:21.0.0.152:8080 user@sandbox-gateway

# Then access locally
http://localhost:8080/stream.mjpg
```

---

## 📺 Method 4: Embed Anywhere

### HTML/Markdown:
```html
<img src="http://21.0.0.152:8080/stream.mjpg" alt="Live Kernel">
```

### React/Next.js:
```jsx
export default function KernelViewer() {
  return (
    <div>
      <h1>Live i9-12900K Kernel</h1>
      <img src="http://21.0.0.152:8080/stream.mjpg" />
    </div>
  );
}
```

### Notion/Obsidian:
```
![Live Stream](http://21.0.0.152:8080/stream.mjpg)
```

---

## 🎛️ Stream Specifications

- **Format**: Motion JPEG (MJPEG) over HTTP
- **Frame Rate**: 2 FPS (can be increased)
- **Resolution**: 1280x720x24
- **Latency**: ~500ms
- **Codec**: JPEG compression (quality 75)
- **Bandwidth**: ~200-400 KB/s per viewer

---

## 🔧 Advanced Options

### Increase Frame Rate:
Edit `/tmp/mjpeg_stream.py` line 10:
```python
FRAME_RATE = 5  # Change from 2 to 5 FPS
```

Then restart:
```bash
pkill -f mjpeg_stream
python3 /tmp/mjpeg_stream.py &
```

### Change Resolution:
Restart Xvfb with different size:
```bash
pkill Xvfb
Xvfb :99 -screen 0 1920x1080x24 &
```

---

## 🌍 Making it Public (Options)

### Option A: ngrok
```bash
ngrok http 21.0.0.152:8080
# Get public URL like: https://abc123.ngrok.io
```

### Option B: Cloudflare Tunnel
```bash
cloudflared tunnel --url http://21.0.0.152:8080
```

### Option C: SSH Reverse Tunnel
```bash
ssh -R 8080:21.0.0.152:8080 user@your-public-server
# Stream accessible at: http://your-public-server:8080
```

---

## 📊 Current Kernel Status

Your kernel is:
- ✅ Booting via GRUB/Multiboot2
- ✅ Serial console working (115200 baud)
- ✅ Running in QEMU compatibility mode
- ✅ Displaying 7-step boot sequence
- ✅ Entering main demonstration loop

---

## 🎯 Quick Test

Test stream from command line:
```bash
# View raw MJPEG data
curl http://21.0.0.152:8080/stream.mjpg | head -100

# Download single frame
curl http://21.0.0.152:8080/stream.mjpg -o frame.jpg
```

---

## 📦 Files Available

1. **Vercel Project**: `/tmp/vercel-viewer/`
   - index.html (beautiful viewer)
   - vercel.json (config)
   - README.md (docs)

2. **Archive**: `/tmp/kernel-viewer.tar.gz` (3.7KB)

3. **Stream Server**: `/tmp/mjpeg_stream.py` (Python 3)

---

## 🔍 Troubleshooting

### Can't connect?
- Check firewall allows port 8080
- Verify routing to 21.0.0.152
- Try SSH tunnel method

### Stream not updating?
```bash
# Check if QEMU is running
ps aux | grep qemu

# Restart stream server
pkill -f mjpeg_stream
python3 /tmp/mjpeg_stream.py &
```

### Want interactive control?
Use noVNC instead:
```
http://21.0.0.152:6080/vnc.html
```

---

**Choose your method and enjoy the live stream! 🚀**
