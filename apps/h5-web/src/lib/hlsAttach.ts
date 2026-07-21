import Hls from 'hls.js'
import { buildPlayUrl, isLiveStatus } from './player'

export type AttachHlsResult = 'hls.js' | 'native' | 'unsupported'

/**
 * Attach an HLS source to a video element.
 * Uses hls.js when MSE is available; otherwise native HLS (e.g. Safari).
 */
export function attachHls(
  video: HTMLVideoElement,
  src: string,
  hlsFactory: typeof Hls = Hls,
): { mode: AttachHlsResult; destroy: () => void } {
  if (hlsFactory.isSupported()) {
    const hls = new hlsFactory()
    hls.loadSource(src)
    hls.attachMedia(video)
    return {
      mode: 'hls.js',
      destroy: () => {
        hls.destroy()
      },
    }
  }
  if (video.canPlayType('application/vnd.apple.mpegurl')) {
    video.src = src
    return {
      mode: 'native',
      destroy: () => {
        video.removeAttribute('src')
        video.load()
      },
    }
  }
  return {
    mode: 'unsupported',
    destroy: () => {},
  }
}

export { buildPlayUrl, isLiveStatus }
