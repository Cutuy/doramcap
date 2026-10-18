// SPDX-License-Identifier: Apache-2.0
// Minimal CO-RE declarations: actual offsets are relocated against host BTF.
typedef unsigned long long __u64;
typedef unsigned int __u32;
typedef int __s32;
typedef unsigned short __u16;
typedef unsigned char __u8;
typedef long long __s64;
typedef __u32 __be32;
typedef __u16 __be16;
typedef __u32 __wsum;
#define BPF_MAP_TYPE_ARRAY 2
#define BPF_ANY 0
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_core_read.h>
#pragma clang attribute push (__attribute__((preserve_access_index)), apply_to = record)
struct task_cputime { __u64 utime; __u64 stime; __u64 sum_exec_runtime; };
struct cgroup_base_stat { struct task_cputime cputime; };
struct cgroup_rstat_cpu { struct cgroup_base_stat subtree_bstat; };
struct kernfs_node { __u64 id; };
struct cgroup;
struct cgroup_subsys_state { struct cgroup *cgroup; struct cgroup_subsys_state *parent; };
struct cgroup { struct cgroup_subsys_state self; struct kernfs_node *kn; struct cgroup_rstat_cpu *rstat_cpu; int level; };
struct seq_file;
struct bpf_iter_meta { struct seq_file *seq; __u64 session_id; __u64 seq_num; };
struct bpf_iter__cgroup { struct bpf_iter_meta *meta; struct cgroup *cgroup; };
#pragma clang attribute pop
extern void cgroup_rstat_flush(struct cgroup *cgrp) __ksym;
struct record { __u64 id, parent_id; __u32 cpu, flags; __u64 runtime_ns, user_ns, system_ns; };
struct control { __u32 capacity, cpus, count, overflow; };
struct { __uint(type, BPF_MAP_TYPE_ARRAY); __uint(max_entries, 1); __type(key, __u32); __type(value, struct control); } control SEC(".maps");
struct { __uint(type, BPF_MAP_TYPE_ARRAY); __uint(max_entries, 1); __type(key, __u32); __type(value, struct record); } records SEC(".maps");

SEC("iter.s/cgroup")
int collect(struct bpf_iter__cgroup *ctx) {
    struct cgroup *cg = ctx->cgroup;
    if (!cg) return 0;
    __u32 zero = 0;
    struct control *ctl = bpf_map_lookup_elem(&control, &zero);
    if (!ctl) return 0;
    if (ctx->meta->seq_num == 0) cgroup_rstat_flush(cg);
    __u64 id = BPF_CORE_READ(cg, kn, id);
    struct cgroup_subsys_state *parent = BPF_CORE_READ(cg, self.parent);
    __u64 parent_id = parent ? BPF_CORE_READ(parent, cgroup, kn, id) : 0;
    int root = BPF_CORE_READ(cg, level) == 0;
    // Hard bound guards verifier complexity and configuration mistakes.
    for (__u32 cpu = 0; cpu < 4096; cpu++) {
        if (cpu >= ctl->cpus) break;
        if (ctl->count >= ctl->capacity) { ctl->overflow++; continue; }
        __u32 index = ctl->count++;
        struct record *r = bpf_map_lookup_elem(&records, &index);
        if (!r) { ctl->overflow++; continue; }
        __builtin_memset(r, 0, sizeof(*r));
        r->id = id; r->parent_id = parent_id; r->cpu = cpu;
        // Root uses global kcpustat, never subtree_bstat; explicitly unavailable.
        if (root) { r->flags = 2; continue; }
        struct cgroup_rstat_cpu *pc = bpf_per_cpu_ptr(cg->rstat_cpu, cpu);
        if (!pc) { r->flags = 4; continue; }
        struct task_cputime time = {};
        if (BPF_CORE_READ_INTO(&time, pc, subtree_bstat.cputime)) { r->flags = 4; continue; }
        r->runtime_ns = time.sum_exec_runtime; r->user_ns = time.utime; r->system_ns = time.stime;
        // Native inclusive runtime; user/system are RAW, before cpu.stat adjustment.
        r->flags = 1 | 8;
    }
    return 0;
}
char LICENSE[] SEC("license") = "GPL";
