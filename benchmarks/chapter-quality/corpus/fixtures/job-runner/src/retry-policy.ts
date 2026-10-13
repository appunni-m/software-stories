export type JobState = "queued" | "running" | "done" | "failed";

export interface JobRecord {
  id: string;
  state: JobState;
  attempts: number;
  result?: string;
  lastError?: string;
}

export function isRetryable(job: JobRecord): boolean {
  return job.state === "failed" && job.attempts < 3;
}
