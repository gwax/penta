/** Local orchestration only: no mutation retry and no implicit gameplay choices. */
export async function sessionInteraction(method, args, connection, command, stats) {
  const { maxWaitMs = 60_000, ...request } = args;
  if (!Number.isInteger(maxWaitMs) || maxWaitMs < 0 || maxWaitMs > 60_000) {
    throw new Error("maxWaitMs must be an integer from 0 to 60000");
  }
  if (method === "next" && Object.keys(request).some(key => key !== "afterEvent")) throw new Error("next accepts only maxWaitMs and full");
  const deadline = performance.now() + maxWaitMs;
  // A zero budget still permits the initial read/submission without a long poll.
  const signal = maxWaitMs ? AbortSignal.timeout(maxWaitMs) : undefined;
  let result = await command(method === "next" ? "observe" : "interact", {
    ...connection, ...request,
    ...(method === "interact" ? { waitMs: Math.min(request.waitMs ?? 25_000, maxWaitMs) } : {}),
  }, { signal });
  const receipt = structuredClone(result.host?.receipt ?? result.receipt ?? null);
  while (result.status === "waiting" && performance.now() < deadline) {
    stats.waitRenewals++;
    try {
      result = await command("wait", { ...connection, ...(request.afterEvent !== undefined ? { afterEvent: request.afterEvent } : {}), waitMs: Math.min(25_000, Math.max(1, Math.floor(deadline - performance.now()))) }, { signal });
    } catch (error) {
      // The submission succeeded. Preserve its receipt even if a later read fails.
      result = { ...result, sessionWait: { reason: signal?.aborted ? "timeout" : "error", error: error.message } };
      break;
    }
  }
  if (receipt) {
    if (result.host) result = { ...result, host: { ...result.host, receipt } };
    else result = { ...result, receipt };
  }
  if (result.status === "waiting" && !result.sessionWait) result = { ...result, sessionWait: { reason: "timeout" } };
  return result;
}
