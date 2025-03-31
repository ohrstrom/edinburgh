export interface Ensemble {
    eid: Number
    label?: string
    short_label?: string
    services: Service[]
}

export interface Service {
    sid: Number
    scid?: Number
    label?: string
    short_label?: string
    dl?: DL
    sls?: SLS
}

export interface DL {
    scid: Number
}

export interface SLS {
    scid: Number
}
