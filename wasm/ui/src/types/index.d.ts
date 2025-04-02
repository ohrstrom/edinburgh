export interface Ensemble {
  eid: number
  label?: string
  short_label?: string
  services: Service[]
}

export interface Service {
  sid: number
  scid?: number
  label?: string
  short_label?: string
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
  md5?: string
  url?: string
}

export interface Volume {
  l: number
  r: number
}
