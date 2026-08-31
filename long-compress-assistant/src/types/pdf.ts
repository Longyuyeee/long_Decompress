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

export type PdfOptimizationMode = 'lossless-organization' | 'compatible-image-optimization'

export interface PdfCompressionExecutionRequest {
  source: string
  destination: string
  mode: PdfOptimizationMode
  confirmedLossyImageChanges: boolean
  preserveMarkOfWeb: boolean
  allowLargerOutput: boolean
}

export interface PdfOptimizationDestinationPlan {
  destination: string
}

export interface PdfStructuralFacts {
  pageCount: number
  encrypted: boolean
  pageMediaBoxes: string[][]
  formFields: Array<{ name: string, fieldType: string }>
  annotations: Array<{ page: number, subtype: string }>
  outlines: Array<{ title: string, page: number | null }>
  attachments: Array<{ key: string, name: string, bytes: number, sha256: string }>
}

export interface PublishedPdfOutput {
  path: string
  inputBytes: number
  outputBytes: number
  savingsRatio: number
  outputSha256: string
  markOfTheWeb: string
  verified: {
    outputBytes: number
    outputSha256: string
    sourceFacts: PdfStructuralFacts
    outputFacts: PdfStructuralFacts
  }
}
