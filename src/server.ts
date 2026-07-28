import type { Register } from '@tanstack/react-router';
import {
  createStartHandler,
  defaultStreamHandler,
} from '@tanstack/react-start/server';
import type { RequestHandler } from '@tanstack/react-start/server';

const fetch = createStartHandler(defaultStreamHandler);

export type ServerEntry = { fetch: RequestHandler<Register> };

export default { fetch };
