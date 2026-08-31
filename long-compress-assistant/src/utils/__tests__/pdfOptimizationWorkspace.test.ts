import { describe, expect, it } from 'vitest'
import { buildPdfConfigurationDraft, defaultPdfOutputPath, isPdfCandidate } from '../pdfOptimizationWorkspace'
import type { PdfInputAnalysisReport } from '@/types/pdf'

const report = (overrides: Partial<PdfInputAnalysisReport> = {}): PdfInputAnalysisReport => ({
  source: 'C:/input/report.pdf', inputBytes: 1024, analysisComplete: true, pageCount: 2,
  encrypted: false, passwordState: 'not-required', hasDigitalSignature: false,
  signatureFieldNames: [], hasFormFields: false, formFieldNames: [], hasAttachments: false,
  attachmentNames: [], outlineCount: 0, warnings: [], blockingReasons: [], ...overrides,
})

describe('pdf optimization workspace policy', () => {
  it('only admits regular PDF paths and always proposes a new output file', () => {
    expect(isPdfCandidate({ path: 'C:/a.pdf', isDirectory: false })).toBe(true)
    expect(isPdfCandidate({ path: 'C:/a.txt', isDirectory: false })).toBe(false)
    expect(defaultPdfOutputPath('C:/a.pdf', 'lossless-organization')).toBe('C:/a.organized.pdf')
    expect(defaultPdfOutputPath('C:/a.pdf', 'compatible-image-optimization')).toBe('C:/a.optimized.pdf')
  })

  it('requires explicit confirmation for lossy image optimization', () => {
    expect(buildPdfConfigurationDraft(report(), 'compatible-image-optimization', false).canFreeze).toBe(false)
    const confirmed = buildPdfConfigurationDraft(report(), 'compatible-image-optimization', true)
    expect(confirmed.canFreeze).toBe(true)
    expect(confirmed.lossy).toBe(true)
    expect(confirmed.sizeReductionGuaranteed).toBe(false)
  })

  it('blocks signed and incompletely analyzed documents', () => {
    expect(buildPdfConfigurationDraft(report({ hasDigitalSignature: true }), 'lossless-organization').canFreeze).toBe(false)
    expect(buildPdfConfigurationDraft(report({ analysisComplete: false }), 'lossless-organization').canFreeze).toBe(false)
  })
})
