import { describe, expect, it } from 'vitest';

import { fitsLimit, tooLargeMessage } from './upload.ts';
import { UploadForm, type UploadRequest } from './upload.svelte.ts';

/** A stand-in for XMLHttpRequest that answers with `status` and `body`
 * after reporting progress. */
function fakeRequest(status: number, body: string, sent: { url?: string; file?: File | null }) {
  return (): UploadRequest => {
    const listeners: Record<string, (event?: ProgressEvent) => void> = {};
    const request: UploadRequest = {
      status: 0,
      responseText: '',
      upload: {
        addEventListener(type, listener) {
          listeners[`upload-${type}`] = listener as (event?: ProgressEvent) => void;
        },
      },
      addEventListener(type, listener) {
        listeners[type] = listener;
      },
      open(_method, url) {
        sent.url = url;
      },
      setRequestHeader() {},
      send(form) {
        sent.file = form.get('log_file') as File | null;
        listeners['upload-progress']?.({
          lengthComputable: true,
          loaded: 50,
          total: 200,
        } as ProgressEvent);
        request.status = status;
        request.responseText = body;
        listeners.load?.();
      },
    };
    return request;
  };
}

describe('UploadForm', () => {
  it('posts the file to /analyze and answers the log page', async () => {
    const sent: { url?: string; file?: File | null } = {};
    const form = new UploadForm(25, fakeRequest(200, '{"redirect":"/logs/abc"}', sent));
    form.logFile = new File(['log'], 'combat.txt');
    expect(await form.submit()).toBe('/logs/abc');
    expect(sent.url).toBe('/analyze');
    expect(sent.file?.name).toBe('combat.txt');
    expect(form.processing).toBe(false);
    expect(form.progress).toBeNull();
  });

  it('shows the field error of a 422', async () => {
    const form = new UploadForm(
      25,
      fakeRequest(422, '{"errors":{"log_file":"The log file field is required."}}', {}),
    );
    expect(await form.submit()).toBeNull();
    expect(form.errors).toEqual({ log_file: 'The log file field is required.' });
  });

  it('turns a refused body into the size message', async () => {
    const form = new UploadForm(25, fakeRequest(413, '<html>', {}));
    await form.submit();
    expect(form.errors.log_file).toBe(
      'The log file field must not be greater than 25600 kilobytes.',
    );
  });

  it('reports a failure', async () => {
    const form = new UploadForm(25, fakeRequest(500, '{"message":"Server Error"}', {}));
    await form.submit();
    expect(form.errors.log_file).toBe('The log file failed to upload.');
  });

  it('checks the size before sending', () => {
    expect(fitsLimit(25 * 1024 * 1024, 25)).toBe(true);
    expect(fitsLimit(25 * 1024 * 1024 + 1, 25)).toBe(false);
    expect(tooLargeMessage(25)).toBe('File is too large. Maximum size is 25MB.');
  });
});
