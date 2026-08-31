export type PdfPasswordState = 'not-required' | 'required' | 'accepted'

export interface PdfInputCandidate {
  path: string
  password?: string | null
}

export interface PdfInputAnalysisReport {
  source: string
  inputBytes: number
  analysisComplete: boolean
  pageCount: number | null
  encrypted: boolean
  passwordState: PdfPasswordState
  hasDigitalSignature: boolean | null
  signatureFieldNames: string[]
  hasFormFields: boolean | null
  formFieldNames: string[]
  hasAttachments: boolean | null
  attachmentNames: string[]
  outlineCount: number | null
  warnings: string[]
  blockingReasons: string[]
}
