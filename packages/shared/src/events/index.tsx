import type { ReactNode } from 'react';
import { createContext, useContext } from 'react';

/**
 * Creates a typed event emitter context for React component trees.
 * Usage: const { EventProvider, useEvent } = createEventEmitter<HandlerType>('name');
 */
export function createEventEmitter<T extends (...args: never[]) => void>(
  _name: string,
) {
  const context = createContext<T | null>(null);

  function EventProvider({
    handler,
    children,
  }: {
    handler: T;
    children: ReactNode;
  }) {
    return <context.Provider value={handler}>{children}</context.Provider>;
  }

  function useEvent() {
    const handler = useContext(context);

    if (!handler) {
      throw new Error(`Event emitter "${_name}" used outside of provider`);
    }

    return handler;
  }

  return { EventProvider, useEvent };
}
