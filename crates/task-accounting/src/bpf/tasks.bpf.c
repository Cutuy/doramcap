// SPDX-License-Identifier: Apache-2.0
typedef unsigned long long __u64;
typedef unsigned int __u32;
typedef int __s32;
typedef unsigned short __u16;
typedef unsigned char __u8;
typedef long long __s64;
typedef __u32 __be32;
typedef __u16 __be16;
typedef __u32 __wsum;
#define BPF_MAP_TYPE_HASH 1
#define BPF_MAP_TYPE_ARRAY 2
#define BPF_NOEXIST 1
#define PF_KTHREAD 0x00200000
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_core_read.h>
#pragma clang attribute push (__attribute__((preserve_access_index)), apply_to = record)
struct thread_info { __u32 cpu; };
struct kernfs_node { __u64 id; };
struct cgroup { struct kernfs_node *kn; };
struct css_set { struct cgroup *dfl_cgrp; };
struct task_struct {
    struct thread_info thread_info;
    int pid, tgid;
    __u64 start_boottime;
    unsigned int flags, policy;
    struct css_set *cgroups;
};
#pragma clang attribute pop
struct bpf_raw_tracepoint_args { __u64 args[0]; };
struct identity {
    __u64 start_boottime_ns, cgroup_id;
    __u32 tid, tgid, cpu, flags;
};
struct counters { __u64 runtime_ns; };
struct losses { __u64 dropped_events, dropped_runtime_ns, identity_errors; };
// HASH uses preallocated entries by default. Never delete cumulative entries
// while attached: task churn eventually exhausts this deliberately bounded map.
struct { __uint(type, BPF_MAP_TYPE_HASH); __uint(max_entries, 1); __type(key, struct identity); __type(value, struct counters); } tasks SEC(".maps");
struct { __uint(type, BPF_MAP_TYPE_ARRAY); __uint(max_entries, 1); __type(key, __u32); __type(value, struct losses); } losses SEC(".maps");
SEC("raw_tp/sched_stat_runtime")
int charge(struct bpf_raw_tracepoint_args *ctx) {
    struct task_struct *task = (void *)ctx->args[0];
    __u64 runtime = ctx->args[1];
    __u32 zero = 0;
    struct losses *lost = bpf_map_lookup_elem(&losses, &zero);
    if (!lost || !runtime) return 0;
    struct identity key = {};
    int pid = 0, tgid = 0;
    unsigned int flags = 0;
    struct css_set *css = 0;
    struct cgroup *cg = 0;
    struct kernfs_node *kn = 0;
    // Runtime updates can execute remotely. The hook CPU is not necessarily
    // the charged CPU; use task_cpu(task)'s native SMP representation.
    if (BPF_CORE_READ_INTO(&pid, task, pid) ||
        BPF_CORE_READ_INTO(&tgid, task, tgid) ||
        BPF_CORE_READ_INTO(&key.start_boottime_ns, task, start_boottime) ||
        BPF_CORE_READ_INTO(&key.cpu, task, thread_info.cpu) ||
        BPF_CORE_READ_INTO(&flags, task, flags) ||
        BPF_CORE_READ_INTO(&css, task, cgroups) || !css ||
        BPF_CORE_READ_INTO(&cg, css, dfl_cgrp) || !cg ||
        BPF_CORE_READ_INTO(&kn, cg, kn) || !kn ||
        BPF_CORE_READ_INTO(&key.cgroup_id, kn, id)) {
        __sync_fetch_and_add(&lost->identity_errors, 1);
        __sync_fetch_and_add(&lost->dropped_events, 1);
        __sync_fetch_and_add(&lost->dropped_runtime_ns, runtime);
        return 0;
    }
    key.tid = pid; key.tgid = tgid;
    key.flags = (flags & PF_KTHREAD) ? 1 : 0;
    struct counters *value = bpf_map_lookup_elem(&tasks, &key);
    if (!value) {
        struct counters initial = {};
        bpf_map_update_elem(&tasks, &key, &initial, BPF_NOEXIST);
        value = bpf_map_lookup_elem(&tasks, &key);
    }
    if (!value) {
        __sync_fetch_and_add(&lost->dropped_events, 1);
        __sync_fetch_and_add(&lost->dropped_runtime_ns, runtime);
        return 0;
    }
    __sync_fetch_and_add(&value->runtime_ns, runtime);
    return 0;
}
char LICENSE[] SEC("license") = "GPL";
