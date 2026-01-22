# i9-12900K Kernel Live Stream Viewer

A real-time MJPEG stream viewer for the bare-metal i9-12900K kernel running in QEMU.

## 🚀 Quick Deploy to Vercel

1. **Install Vercel CLI** (if you haven't already):
   ```bash
   npm install -g vercel
   ```

2. **Deploy this directory**:
   ```bash
   cd /tmp/vercel-viewer
   vercel --prod
   ```

3. **Access your live stream** at the provided Vercel URL!

## 📡 Stream Endpoints

### Direct MJPEG Stream
```
http://21.0.0.152:8080/stream.mjpg
```

### Interactive noVNC Console
```
http://21.0.0.152:6080/vnc.html
```

### Stream Viewer Page
```
http://21.0.0.152:8080/
```

## 🔧 Network Requirements

The stream server runs on a **private network** at `21.0.0.152`. You'll need one of:

1. **VPN/Direct Access** to the 21.0.0.0/24 network
2. **SSH Tunnel**:
   ```bash
   ssh -L 8080:21.0.0.152:8080 user@gateway
   ```
   Then access: `http://localhost:8080/stream.mjpg`

3. **Reverse Proxy** (e.g., ngrok, Cloudflare Tunnel):
   ```bash
   ngrok http 21.0.0.152:8080
   ```

## 📦 What's Included

- `index.html` - Beautiful viewer interface with real-time stream
- `vercel.json` - Vercel deployment configuration
- Automatic error handling and reconnection
- Responsive design for mobile/desktop

## 🎨 Features

- ✅ Real-time MJPEG stream at 2 FPS
- ✅ Auto-reconnect on connection loss
- ✅ System information display
- ✅ Embed code snippets
- ✅ Matrix-style terminal aesthetic
- ✅ Mobile responsive

## 🔌 Embed in Your Own Site

### Simple Image Tag
```html
<img src="http://21.0.0.152:8080/stream.mjpg" alt="Kernel Stream">
```

### Full Viewer in iframe
```html
<iframe src="http://21.0.0.152:8080/" width="1280" height="800"></iframe>
```

### React Component
```jsx
export default function KernelStream() {
  return (
    <div>
      <h1>Live Kernel Stream</h1>
      <img
        src="http://21.0.0.152:8080/stream.mjpg"
        alt="Kernel Stream"
        style={{ width: '100%', maxWidth: '1280px' }}
      />
    </div>
  );
}
```

## 🛠️ Technical Details

- **Format**: Motion JPEG over HTTP
- **Frame Rate**: 2 FPS (configurable in server)
- **Resolution**: 1280x720x24
- **Latency**: ~500ms
- **Source**: Xvfb :99 → scrot → MJPEG stream

## 📝 Manual Deployment (Alternative)

If not using Vercel:

1. Host `index.html` on any static hosting service (Netlify, GitHub Pages, etc.)
2. Ensure CORS is configured to allow access from your domain
3. Users will need network access to `21.0.0.152:8080`

## 🔒 Security Notes

- No authentication on stream (add if needed)
- Stream is HTTP (not HTTPS) - use tunnel for encryption
- Designed for development/demo purposes

---

**Built with**: Python 3, MJPEG, HTML5, Vercel
**Project**: Silent-Breath-Online Bare-Metal Kernel Development
