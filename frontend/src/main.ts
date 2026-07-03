import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'

// Demo builds (VITE_MOCK_BUILD=1, see `npm run build:demo`) have no backend, so
// install the in-browser mock before anything can issue a request. The env flag
// is a compile-time constant, so a normal production build dead-code-eliminates
// this branch and never bundles the mock.
async function bootstrap() {
  if (import.meta.env.VITE_MOCK_BUILD) {
    const { installMockBackend } = await import('../mock/browser')
    installMockBackend()
  }

  return mount(App, {
    target: document.getElementById('app')!,
  })
}

export default bootstrap()
