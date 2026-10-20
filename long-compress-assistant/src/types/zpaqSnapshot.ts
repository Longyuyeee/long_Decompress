export interface ZpaqSnapshot { version: number; date: number }
export interface ZpaqSnapshotInfo extends ZpaqSnapshot { updates: number; deletes: number }
export interface ZpaqHistory { selected: ZpaqSnapshot; snapshots: ZpaqSnapshotInfo[] }
export const isZpaqSnapshot = (value: unknown): value is ZpaqSnapshot => {
  const item = value as ZpaqSnapshot | null
  return !!item && Number.isInteger(item.version) && item.version > 0 && item.version <= 9999999
    && Number.isInteger(item.date) && item.date >= 19000101000000 && item.date <= 29991231235959
}
export const snapshotArguments = (snapshot?: ZpaqSnapshot) => snapshot
  ? { zpaqSnapshot: { version: snapshot.version, date: snapshot.date } } : {}
export const snapshotKey = (snapshot?: ZpaqSnapshot) => snapshot ? `${snapshot.version}:${snapshot.date}` : 'latest'
export const snapshotDate = (date: number) => {
  const s = String(date)
  return `${s.slice(0,4)}-${s.slice(4,6)}-${s.slice(6,8)} ${s.slice(8,10)}:${s.slice(10,12)}:${s.slice(12)} UTC`
}
