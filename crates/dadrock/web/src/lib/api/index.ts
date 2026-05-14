import { MockApi } from './mock';
import { RealApi } from './real';
import type { DadrApi } from './types';

export type * from './types';
export type * from './schemas';

export function createApi(): DadrApi {
  return import.meta.env.VITE_DADR_MOCK === '1' ? new MockApi() : new RealApi();
}
