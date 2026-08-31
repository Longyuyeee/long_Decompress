import type { PdfInputAnalysisReport } from '@/types/pdf'

export type PdfOptimizationMode = 'lossless-organization' | 'compatible-image-optimization'

export interface PdfConfigurationDraft {
  mode: PdfOptimizationMode
  modeLabel: string
  lossy: boolean
  proposedOutput: string
  sourceMutationAllowed: false
  sizeReductionGuaranteed: false
  requiresExplicitConfirmation: boolean
  warnings: string[]
  blockingReasons: string[]
  canFreeze: boolean
}

export const PDF_MODE_FACTS = {
  'lossless-organization': {
    label: '无损整理',
    lossy: false,
    outputSuffix: '.organized.pdf',
  },
  'compatible-image-optimization': {
    label: '兼容图片优化',
    lossy: true,
    outputSuffix: '.optimized.pdf',
  },
} as const

export const isPdfCandidate = (candidate: { path?: string, isDirectory?: boolean }) =>
  !!candidate.path && !candidate.isDirectory && candidate.path.toLowerCase().endsWith('.pdf')

export const defaultPdfOutputPath = (source: string, mode: PdfOptimizationMode) => {
  const suffix = PDF_MODE_FACTS[mode].outputSuffix
  return source.toLowerCase().endsWith('.pdf') ? `${source.slice(0, -4)}${suffix}` : `${source}${suffix}`
}

export const buildPdfConfigurationDraft = (
  report: PdfInputAnalysisReport,
  mode: PdfOptimizationMode,
  riskConfirmed = false,
): PdfConfigurationDraft => {
  const facts = PDF_MODE_FACTS[mode]
  const blockingReasons = [...report.blockingReasons]
  if (!report.analysisComplete && !blockingReasons.length) blockingReasons.push('PDF_ANALYSIS_INCOMPLETE')
  if (report.hasDigitalSignature === true && !blockingReasons.some(reason => reason.includes('SIGN'))) {
    blockingReasons.push('PDF_DIGITAL_SIGNATURE_EXECUTION_BLOCKED')
  }
  const requiresExplicitConfirmation = facts.lossy
  return {
    mode,
    modeLabel: facts.label,
    lossy: facts.lossy,
    proposedOutput: defaultPdfOutputPath(report.source, mode),
    sourceMutationAllowed: false,
    sizeReductionGuaranteed: false,
    requiresExplicitConfirmation,
    warnings: [
      ...report.warnings,
      ...(facts.lossy ? ['图片可能被重新编码，像素与编码数据可能发生不可逆变化。'] : []),
      '输出大小取决于原文件结构，不保证一定变小。',
    ],
    blockingReasons,
    canFreeze: report.analysisComplete && blockingReasons.length === 0 && (!requiresExplicitConfirmation || riskConfirmed),
  }
}
