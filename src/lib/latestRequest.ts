/** Invalidate earlier async work, including its error and loading updates. */
export function latestRequest() {
  let generation = 0;
  return {
    begin() {
      const current = ++generation;
      return () => current === generation;
    },
    invalidate() { generation++; },
  };
}
