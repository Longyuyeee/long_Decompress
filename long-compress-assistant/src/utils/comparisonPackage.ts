import {isZipPasswordEncoding} from '@/types/zipPassword'
import type { ComparisonPackageReference } from '@/types/zipEdit'
export function copyComparisonReference(value: unknown): ComparisonPackageReference | undefined {
 const row=value as ComparisonPackageReference|undefined
 if(!row||typeof row.path!=='string'||!row.path||!row.source||typeof row.source.path!=='string'||!row.source.path||!['directory','zip'].includes(row.source.kind)||!/^[0-9a-f]{64}$/i.test(row.source.revision))return undefined
 return {path:row.path,...(isZipPasswordEncoding(row.encoding)?{encoding:row.encoding}:{}),source:{path:row.source.path,kind:row.source.kind,revision:row.source.revision,...(row.source.encrypted?{encrypted:true}:{})}}
}
