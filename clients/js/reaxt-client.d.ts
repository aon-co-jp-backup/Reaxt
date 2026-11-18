export class ReaxtError extends Error {
  status: number;
  code: string;
  constructor(status: number, code: string, message: string);
}
export interface ActionInfo { service: string; method: string; path: string }
export interface ClientOptions {
  baseUrl?: string;
  headers?: Record<string, string>;
  fetch?: typeof fetch;
}
export interface ReaxtClient {
  call<I = unknown, O = unknown>(service: string, method: string, input?: I): Promise<O>;
  manifest(): Promise<ActionInfo[]>;
  defineAction<I = null, O = unknown>(service: string, method: string): (input?: I) => Promise<O>;
}
export function createClient(opts?: ClientOptions): ReaxtClient;
