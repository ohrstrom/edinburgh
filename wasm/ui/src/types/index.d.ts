export interface Ensemble {
  eid: number
  label?: string
  short_label?: string
  services: Service[]
}

export interface Subchannel {
  id: number
  start: number
  size: number
  bitrate: number
  pl: string
}

export interface Service {
  sid: number
  scid?: number
  label?: string
  short_label?: string
  subchannel?: Subchannel
  isPlayting: boolean
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
}

//
export interface Level {
  l: number
  r: number
}

//
export type PlayerState = 'stopped' | 'playing' | 'paused'