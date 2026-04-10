import Bowser from 'bowser'
import type * as Types from '@/types'

const parser = Bowser.getParser(window.navigator.userAgent)

const BROWSER_SUPPORT = {
  macos: {
    safari: '>=15',
  },
  desktop: {
    chrome: '>=100',
    firefox: '>=100',
    opera: '>=80',
  },
  mobile: {
    chrome: '>=100',
    firefox: '>=100',
    'android browser': '>=14',
  },
}

const BROWSER_CAPABILITIES = [
  'OfflineAudioContext',
  'AudioContext',
  'OfflineAudioContext',
  'AudioWorkletNode',
  'GainNode',
  'AudioDecoder',
  'EncodedAudioChunk',
]

const useBrowser = () => {
  const missing: Array<string> = []

  BROWSER_CAPABILITIES.forEach((capability: string) => {
    if (typeof (window as never)[capability] === 'undefined') {
      missing.push(capability)
    }
  })

  const browser: Types.Browser = {
    isSupported: (missing.length === 0 && parser.satisfies(BROWSER_SUPPORT)) || false,
    os: parser.getOSName(true),
    platform: parser.getBrowserName(true),
    name: parser.getPlatformType(true),
    version: parser.getBrowserVersion() || 'unknown',
    missing: missing,
  }

  return {
    browser,
  }
}

export { useBrowser }
