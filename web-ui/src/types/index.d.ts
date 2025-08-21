export interface Ensemble {
  eid: number
  label?: string
  short_label?: string
  services: Service[]
  subchannels: Subchannel[]
}

export interface Subchannel {
  id: number
  start: number
  size: number
  bitrate: number
  pl: string
}

export interface ServiceComponent {
  scid: number
  subchannel_id?: number
  language?: string
  user_apps?: string[] // or Types.UserApplication if enum-based
}

export interface Service {
  sid: number
  label?: string
  short_label?: string
  // subchannel?: Subchannel
  components: ServiceComponent[]
  audioFormat?: AudioFormat
  isPlaying: boolean
  dl?: DL
  sls?: SLS
}

interface DLPlusTag {
  kind: string
  value: string
}

export interface DL {
  scid: number
  label?: string
  dl_plus?: DLPlusTag[]
}

export interface SLS {
  scid: number
  mimetype?: string
  data?: Byte[]
  md5?: string
  url?: string
  width?: number
  height?: number
}

export interface AACSegment {
  scid: number
  audio_format: AudioFormat,
  frames: ArrayBuffer[]
}

export interface AudioFormat {
  sbr: boolean
  ps: boolean
  codec: string
  au_count: number
  samplerate: number
  bitrate: number
  channels: number
  asc: ArrayBuffer | Uint8Array
}

//
export interface Level {
  l: number
  r: number
}

//
export type PlayerState = 'stopped' | 'playing' | 'paused'

//
export interface Browser {
  isSupported: boolean
  os: string
  platform: string
  name: string
  version: string
  missing: Array<string>
}
