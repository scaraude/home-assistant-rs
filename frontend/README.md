# Home Automation Frontend

Lightweight Svelte frontend for the home-automation-rs Raspberry Pi project.

## Features

- **Minimal & Lightweight**: ~82 KB gzipped bundle size
- **Real-time Updates**: 15-second polling interval
- **Responsive Design**: Works on desktop and mobile
- **Chart.js Graphs**: Temperature and humidity visualization
- **Material Design Icons**: Clean, professional UI
- **TypeScript**: Full type safety

## Tech Stack

- **Svelte 5** - Reactive UI framework
- **TypeScript** - Type-safe development
- **Vite** - Fast build tool with optimizations
- **Chart.js** - Lightweight charting library
- **Material Design** - Icon system

## Project Structure

```
frontend/
├── src/
│   ├── App.svelte              # Main app container
│   ├── lib/
│   │   ├── api.ts              # API client for backend
│   │   ├── SensorCard.svelte   # Card component
│   │   └── SensorGraph.svelte # Chart.js wrapper
│   ├── main.ts                 # Entry point
│   └── app.css                 # Global styles
├── vite.config.ts              # Build configuration
└── package.json
```

## Development

### Prerequisites

- Node.js 18+ and npm
- Running home-automation-rs backend (on Pi or localhost:8082)

### Setup

```bash
cd frontend
npm install
```

### Dev Server

```bash
npm run dev
```

The dev server will start at `http://localhost:5173` and proxy API requests to `http://localhost:8082`.

### Build for Production

```bash
npm run build
```

This will:
- Compile TypeScript and Svelte components
- Minify and optimize JavaScript/CSS
- Output to `../static/` directory (for Rust backend)
- Generate production-ready assets

### Build Output

- **Bundle Size**: ~248 KB uncompressed, ~82 KB gzipped
- **Assets**:
  - `index.html` - Entry point
  - `assets/index-*.js` - JavaScript bundle
  - `assets/index-*.css` - Stylesheet

## Configuration

### API Endpoints

The frontend expects these API endpoints:

- `GET /api/sensors` - Returns array of sensor IDs
- `GET /api/readings?sensor_id=<id>&hours=<hours>` - Returns sensor readings

### Polling Interval

To change the polling interval, edit `src/App.svelte`:

```typescript
intervalId = window.setInterval(() => {
  loadData();
}, 15000); // Change to desired interval in milliseconds
```

### Dev Server Proxy

To change the backend URL, edit `vite.config.ts`:

```typescript
proxy: {
  '/api': {
    target: 'http://localhost:8082', // Change to your backend URL
    changeOrigin: true,
  },
}
```

## Deployment

### Manual Deployment (Current)

1. Build on Mac: `npm run build`
2. Files are automatically placed in `../static/`
3. Rebuild Docker image: `docker compose build home-automation-rs`
4. Deploy: `docker compose up -d`

### Automated Deployment (Future)

GitHub Actions workflow can be added to:
1. Build frontend on push
2. Copy assets to Docker build context
3. Trigger Pi deployment

## Performance

- **Initial Load**: < 1 second on fast connection
- **Bundle Size**: 82 KB gzipped (very lightweight)
- **Polling**: 15-second intervals (low network overhead)
- **Memory**: ~10-20 MB in browser
- **Chart Rendering**: Hardware-accelerated Canvas

## Browser Support

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+
- Mobile browsers (iOS Safari, Chrome Android)

## Customization

### Changing Colors

Edit component styles or `src/app.css`:

```css
:root {
  --primary: #3b82f6;
  --secondary: #22c55e;
  /* Add your custom colors */
}
```

### Adding More Chart Types

Install additional Chart.js controllers and register them in `src/lib/graphs/SensorGraph.svelte`.

### Changing Icons

Icons are inline SVG from Material Design. To change:
1. Find icons at https://fonts.google.com/icons
2. Export as SVG
3. Replace in component

## Troubleshooting

### Build Fails

```bash
# Clean install
rm -rf node_modules package-lock.json
npm install
```

### API Errors

Check that:
1. Backend is running on port 8082
2. CORS is enabled (if different domain)
3. API endpoints return correct JSON

### Dev Server Proxy Not Working

Verify `vite.config.ts` proxy target matches your backend URL.

## License

Same as parent project
