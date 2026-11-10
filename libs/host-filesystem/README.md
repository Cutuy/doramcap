# Host filesystem checks

Identify live host shared memory and host-directory aliases through native filesystem metadata.
The workload backends share these checks when preparing mounts and recording files.

See the [API documentation](src/lib.rs) for inputs and errors.
