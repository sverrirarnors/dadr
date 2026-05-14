import {
  candidateTemplateSchema,
  commandEnvelopeSchema,
  mutationResponseSchema,
  recordLifecycleSchema,
  recordSourceSchema,
  snapshotSchema,
  taskListingSchema,
  taskPackageSchema,
  taskResultSchema,
  taskRunLogEntrySchema,
  taskRunSchema,
  type CandidateTemplate,
  type MutationResponse,
  type RecordLifecycle,
  type RecordSource,
  type Snapshot,
  type TaskListing,
  type TaskPackage,
  type TaskResult,
  type TaskRun,
  type TaskRunLogEntry,
  type ValidationTarget,
} from './schemas';
import type {
  ApplyRecordRevisionRequest,
  CreateCandidateRequest,
  DadrApi,
  DraftCandidateRequest,
  RunTaskRequest,
  SupersedeRecordsRequest,
} from './types';

type Fetcher = typeof fetch;

async function apiError(response: Response, fallback: string): Promise<Error> {
  const detail = await responseErrorDetail(response);
  return new Error(detail ? `${fallback}: ${detail}` : fallback);
}

async function responseErrorDetail(response: Response): Promise<string> {
  let text = '';
  try {
    text = await response.text();
  } catch {
    return '';
  }
  const trimmed = text.trim();
  if (!trimmed) return '';

  try {
    const parsed = JSON.parse(trimmed) as unknown;
    if (parsed && typeof parsed === 'object' && 'error' in parsed) {
      const error = (parsed as { error?: unknown }).error;
      if (typeof error === 'string' && error.trim()) return error.trim();
    }
  } catch {
    // Fall through to the raw response text for non-JSON error bodies.
  }

  return trimmed;
}

export class RealApi implements DadrApi {
  constructor(
    private readonly baseUrl: string = '',
    private readonly fetcher: Fetcher = fetch,
  ) {}

  async getSnapshot(): Promise<Snapshot> {
    const response = await this.fetcher(`${this.baseUrl}/api/project`, {
      method: 'GET',
      headers: { accept: 'application/json' },
    });
    if (!response.ok) {
      throw await apiError(response, `GET /api/project failed with ${response.status}`);
    }
    return snapshotSchema.parse(await response.json());
  }

  async initializeProject(): Promise<MutationResponse> {
    const response = await this.fetcher(`${this.baseUrl}/api/init`, {
      method: 'POST',
      headers: {
        accept: 'application/json',
        'content-type': 'application/json',
      },
      body: JSON.stringify({}),
    });
    if (!response.ok) {
      throw await apiError(response, `POST /api/init failed with ${response.status}`);
    }
    return mutationResponseSchema.parse(await response.json());
  }

  async getRecordSource(id: string): Promise<RecordSource> {
    const response = await this.fetcher(`${this.baseUrl}/api/records/${encodeURIComponent(id)}/source`, {
      method: 'GET',
      headers: { accept: 'application/json' },
    });
    if (!response.ok) {
      throw await apiError(response, `GET /api/records/${id}/source failed with ${response.status}`);
    }
    return recordSourceSchema.parse(await response.json());
  }

  async getRecordLifecycle(id: string): Promise<RecordLifecycle> {
    const response = await this.fetcher(`${this.baseUrl}/api/records/${encodeURIComponent(id)}/status`, {
      method: 'GET',
      headers: { accept: 'application/json' },
    });
    if (!response.ok) {
      throw await apiError(response, `GET /api/records/${id}/status failed with ${response.status}`);
    }
    return recordLifecycleSchema.parse(await response.json());
  }

  async saveRecordSource(id: string, content: string): Promise<MutationResponse> {
    const response = await this.fetcher(`${this.baseUrl}/api/records/${encodeURIComponent(id)}/source`, {
      method: 'POST',
      headers: {
        accept: 'application/json',
        'content-type': 'application/json',
      },
      body: JSON.stringify({ content }),
    });
    if (!response.ok) {
      throw await apiError(response, `POST /api/records/${id}/source failed with ${response.status}`);
    }
    return mutationResponseSchema.parse(await response.json());
  }

  async runValidation(target: ValidationTarget): Promise<MutationResponse> {
    const response = await this.fetcher(`${this.baseUrl}/api/validate/${target}`, {
      method: 'POST',
      headers: {
        accept: 'application/json',
        'content-type': 'application/json',
      },
      body: JSON.stringify({}),
    });
    if (!response.ok) {
      throw await apiError(response, `POST /api/validate/${target} failed with ${response.status}`);
    }
    return mutationResponseSchema.parse(await response.json());
  }

  async getCandidateTemplate(): Promise<CandidateTemplate> {
    const response = await this.fetcher(`${this.baseUrl}/api/templates/candidate`, {
      method: 'GET',
      headers: { accept: 'application/json' },
    });
    if (!response.ok) {
      throw await apiError(response, `GET /api/templates/candidate failed with ${response.status}`);
    }
    return candidateTemplateSchema.parse(await response.json());
  }

  async createCandidate(request: CreateCandidateRequest): Promise<MutationResponse> {
    const response = await this.fetcher(`${this.baseUrl}/api/records`, {
      method: 'POST',
      headers: {
        accept: 'application/json',
        'content-type': 'application/json',
      },
      body: JSON.stringify(request),
    });
    if (!response.ok) {
      throw await apiError(response, `POST /api/records failed with ${response.status}`);
    }
    return mutationResponseSchema.parse(await response.json());
  }

  async applyRecordRevision(request: ApplyRecordRevisionRequest): Promise<MutationResponse> {
    const { record_id, ...body } = request;
    const response = await this.fetcher(
      `${this.baseUrl}/api/records/${encodeURIComponent(record_id)}/revision`,
      {
        method: 'POST',
        headers: {
          accept: 'application/json',
          'content-type': 'application/json',
        },
        body: JSON.stringify(body),
      },
    );
    if (!response.ok) {
      throw await apiError(response, `POST /api/records/${record_id}/revision failed with ${response.status}`);
    }
    return mutationResponseSchema.parse(await response.json());
  }

  async supersedeRecords(request: SupersedeRecordsRequest): Promise<MutationResponse> {
    if (!request.to.length) {
      throw new Error('Select at least one superseding record');
    }
    const response = await this.fetcher(`${this.baseUrl}/api/records/supersede`, {
      method: 'POST',
      headers: {
        accept: 'application/json',
        'content-type': 'application/json',
      },
      body: JSON.stringify(request),
    });
    if (!response.ok) {
      throw await apiError(response, `POST /api/records/supersede failed with ${response.status}`);
    }
    return mutationResponseSchema.parse(await response.json());
  }

  async draftCandidate(request: DraftCandidateRequest): Promise<MutationResponse> {
    const response = await this.fetcher(`${this.baseUrl}/api/agent/draft`, {
      method: 'POST',
      headers: {
        accept: 'application/json',
        'content-type': 'application/json',
      },
      body: JSON.stringify(request),
    });
    if (!response.ok) {
      throw await apiError(response, `POST /api/agent/draft failed with ${response.status}`);
    }
    return mutationResponseSchema.parse(await response.json());
  }

  async acceptCandidate(id: string): Promise<MutationResponse> {
    return this.candidateAction(id, 'accept');
  }

  async rejectCandidate(id: string): Promise<MutationResponse> {
    return this.candidateAction(id, 'reject');
  }

  async listTasks(): Promise<TaskListing> {
    const response = await this.fetcher(`${this.baseUrl}/api/tasks`, {
      method: 'GET',
      headers: { accept: 'application/json' },
    });
    if (!response.ok) {
      throw await apiError(response, `GET /api/tasks failed with ${response.status}`);
    }
    return commandEnvelopeSchema(taskListingSchema).parse(await response.json()).data;
  }

  async getTask(id: string): Promise<TaskPackage> {
    const response = await this.fetcher(`${this.baseUrl}/api/tasks/${encodeURIComponent(id)}`, {
      method: 'GET',
      headers: { accept: 'application/json' },
    });
    if (!response.ok) {
      throw await apiError(response, `GET /api/tasks/${id} failed with ${response.status}`);
    }
    return commandEnvelopeSchema(taskPackageSchema).parse(await response.json()).data;
  }

  async listTaskRuns(): Promise<TaskRun[]> {
    const response = await this.fetcher(`${this.baseUrl}/api/task-runs`, {
      method: 'GET',
      headers: { accept: 'application/json' },
    });
    if (!response.ok) {
      throw await apiError(response, `GET /api/task-runs failed with ${response.status}`);
    }
    return commandEnvelopeSchema(taskRunSchema.array()).parse(await response.json()).data;
  }

  async getTaskResult(id: string): Promise<TaskResult> {
    const response = await this.fetcher(`${this.baseUrl}/api/task-runs/${encodeURIComponent(id)}`, {
      method: 'GET',
      headers: { accept: 'application/json' },
    });
    if (!response.ok) {
      throw await apiError(response, `GET /api/task-runs/${id} failed with ${response.status}`);
    }
    return commandEnvelopeSchema(taskResultSchema).parse(await response.json()).data;
  }

  async getTaskLog(id: string): Promise<TaskRunLogEntry[]> {
    const response = await this.fetcher(`${this.baseUrl}/api/task-runs/${encodeURIComponent(id)}/log`, {
      method: 'GET',
      headers: { accept: 'application/json' },
    });
    if (!response.ok) {
      throw await apiError(response, `GET /api/task-runs/${id}/log failed with ${response.status}`);
    }
    return commandEnvelopeSchema(taskRunLogEntrySchema.array()).parse(await response.json()).data;
  }

  async runTask(request: RunTaskRequest): Promise<TaskRun> {
    const response = await this.fetcher(`${this.baseUrl}/api/tasks/run`, {
      method: 'POST',
      headers: {
        accept: 'application/json',
        'content-type': 'application/json',
      },
      body: JSON.stringify(request),
    });
    if (!response.ok) {
      throw await apiError(response, `POST /api/tasks/run failed with ${response.status}`);
    }
    return commandEnvelopeSchema(taskRunSchema).parse(await response.json()).data;
  }

  async cancelTaskRun(id: string): Promise<TaskRun> {
    const response = await this.fetcher(`${this.baseUrl}/api/task-runs/${encodeURIComponent(id)}/cancel`, {
      method: 'POST',
      headers: {
        accept: 'application/json',
        'content-type': 'application/json',
      },
      body: JSON.stringify({}),
    });
    if (!response.ok) {
      throw await apiError(response, `POST /api/task-runs/${id}/cancel failed with ${response.status}`);
    }
    return commandEnvelopeSchema(taskRunSchema).parse(await response.json()).data;
  }

  async clearTaskHistory(): Promise<MutationResponse> {
    const response = await this.fetcher(`${this.baseUrl}/api/task-runs/clear`, {
      method: 'POST',
      headers: {
        accept: 'application/json',
        'content-type': 'application/json',
      },
      body: JSON.stringify({}),
    });
    if (!response.ok) {
      throw await apiError(response, `POST /api/task-runs/clear failed with ${response.status}`);
    }
    return mutationResponseSchema.parse(await response.json());
  }

  private async candidateAction(id: string, action: 'accept' | 'reject'): Promise<MutationResponse> {
    const response = await this.fetcher(
      `${this.baseUrl}/api/candidates/${encodeURIComponent(id)}/${action}`,
      {
        method: 'POST',
        headers: {
          accept: 'application/json',
          'content-type': 'application/json',
        },
        body: JSON.stringify({}),
      },
    );
    if (!response.ok) {
      throw await apiError(response, `POST /api/candidates/${id}/${action} failed with ${response.status}`);
    }
    return mutationResponseSchema.parse(await response.json());
  }
}
