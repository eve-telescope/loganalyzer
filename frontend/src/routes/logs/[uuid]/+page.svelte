<script lang="ts">
  // The page is not ported yet; its load already asks the api.
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
</script>

<pre>{JSON.stringify(data)}</pre>
