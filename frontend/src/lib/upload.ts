// The upload form's client side: the size check before sending, and the
// messages a failed upload shows.

/** The message of a file over the limit, checked before it is sent. */
export function tooLargeMessage(maxSizeMb: number): string {
  return `File is too large. Maximum size is ${maxSizeMb}MB.`;
}

/** Whether a file is within `maxSizeMb` (MiB). */
export function fitsLimit(size: number, maxSizeMb: number): boolean {
  return size <= maxSizeMb * 1024 * 1024;
}

/** The message for an answer that is not the api's 200 or 422: a body
 * the proxy refused, or a failure. */
export function failureMessage(status: number, maxSizeMb: number): string {
  if (status === 413) {
    return `The log file field must not be greater than ${maxSizeMb * 1024} kilobytes.`;
  }
  return 'The log file failed to upload.';
}
