import { describe, expect, it } from 'vitest'
import { createImageMediaMetricsV1, createMeasuredTaskMetricsV1, createVerifiedImageTaskMetricsV1 } from '../taskMetrics'

describe('createMeasuredTaskMetricsV1', () => {
  it('derives final savings only from normalized filesystem byte facts', () => {
    expect(createMeasuredTaskMetricsV1(1_000.4, 600.4)).toEqual({
      schemaVersion: 1,
      inputBytes: 1000,
      outputBytes: 600,
      savingsRatio: 0.4,
    })
  })

  it('does not serialize non-finite or negative values as measured facts', () => {
    expect(createMeasuredTaskMetricsV1(Number.POSITIVE_INFINITY, Number.NaN)).toEqual({
      schemaVersion: 1,
      inputBytes: 0,
      outputBytes: 0,
      savingsRatio: 0,
    })
    expect(createMeasuredTaskMetricsV1(-1, -20).inputBytes).toBe(0)
  })

  it('keeps verified image input and output facts separate', () => {
    const input = {
      format: 'jpeg' as const,
      encodedWidth: 640,
      encodedHeight: 360,
      visibleWidth: 360,
      visibleHeight: 640,
      orientation: 6,
      frameCount: 1,
      hasAlpha: false,
    }
    const output = {
      format: 'png' as const,
      encodedWidth: 360,
      encodedHeight: 640,
      visibleWidth: 360,
      visibleHeight: 640,
      orientation: 1,
      frameCount: 1,
      hasAlpha: false,
    }

    const media = createImageMediaMetricsV1(input, output)

    expect(media.image?.input).toEqual(input)
    expect(media.image?.output).toEqual(output)
    expect(media.image?.input).not.toBe(input)
    expect(media.image?.output).not.toBe(output)
  })

  it('derives image byte metrics from verified backend facts', () => {
    const input = {
      format: 'png' as const, encodedBytes: 1546, encodedWidth: 256, encodedHeight: 256,
      visibleWidth: 256, visibleHeight: 256, orientation: 1, frameCount: 1, hasAlpha: true,
    }
    const output = { ...input, encodedBytes: 1000 }

    expect(createVerifiedImageTaskMetricsV1(input, output)).toEqual(expect.objectContaining({
      inputBytes: 1546,
      outputBytes: 1000,
      savingsRatio: 546 / 1546,
      media: { image: {
        input: expect.not.objectContaining({ encodedBytes: expect.anything() }),
        output: expect.not.objectContaining({ encodedBytes: expect.anything() }),
      } },
    }))
  })
})
