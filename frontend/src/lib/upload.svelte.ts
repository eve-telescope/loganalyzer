// The upload form's state (`errors`, `processing`, `progress`). The file goes up with XMLHttpRequest,
// which reports upload progress (fetch does not).
import { failureMessage } from './upload.ts';

export interface UploadProgress {
  percentage: number;
}

/** What the form needs of XMLHttpRequest, so tests can stand in for it. */
export interface UploadRequest {
  open(method: string, url: string): void;
  setRequestHeader(name: string, value: string): void;
  send(body: FormData): void;
  upload: { addEventListener(type: 'progress', listener: (event: ProgressEvent) => void): void };
  addEventListener(type: 'load' | 'error', listener: () => void): void;
  status: number;
  responseText: string;
}

export class UploadForm {
  logFile = $state<File | null>(null);
  errors = $state<{ log_file?: string }>({});
  processing = $state(false);
  progress = $state<UploadProgress | null>(null);

  #maxSizeMb: number;
  #request: () => UploadRequest;

  constructor(
    maxSizeMb: number,
    request: () => UploadRequest = () => new XMLHttpRequest() as unknown as UploadRequest,
  ) {
    this.#maxSizeMb = maxSizeMb;
    this.#request = request;
  }

  setError(message: string): void {
    this.errors = { log_file: message };
  }

  clearErrors(): void {
    this.errors = {};
  }

  /** Sends the file; resolves with the page to go to, or null when the
   * upload failed (the errors say why). */
  submit(): Promise<string | null> {
    const file = this.logFile;
    const body = new FormData();
    if (file) {
      body.append('log_file', file);
    }
    this.processing = true;
    this.progress = null;

    return new Promise((resolve) => {
      const request = this.#request();
      const finish = (redirect: string | null) => {
        this.processing = false;
        this.progress = null;
        resolve(redirect);
      };
      request.open('POST', '/analyze');
      request.setRequestHeader('Accept', 'application/json');
      request.upload.addEventListener('progress', (event) => {
        if (event.lengthComputable) {
          this.progress = { percentage: Math.round((event.loaded / event.total) * 100) };
        }
      });
      request.addEventListener('load', () => {
        let answer: { redirect?: string; errors?: { log_file?: string } } = {};
        try {
          answer = JSON.parse(request.responseText);
        } catch {
          // Not the api's answer (a proxy's error page).
        }
        if (request.status === 200 && answer.redirect) {
          this.clearErrors();
          finish(answer.redirect);
        } else if (request.status === 422 && answer.errors) {
          this.errors = answer.errors;
          finish(null);
        } else {
          this.setError(failureMessage(request.status, this.#maxSizeMb));
          finish(null);
        }
      });
      request.addEventListener('error', () => {
        this.setError(failureMessage(0, this.#maxSizeMb));
        finish(null);
      });
      request.send(body);
    });
  }
}
