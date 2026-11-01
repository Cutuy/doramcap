// SPDX-License-Identifier: GPL-2.0-only
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
struct bpf_iter_num { __u64 __opaque[1]; } __attribute__((aligned(8)));
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_core_read.h>
#pragma clang attribute push (__attribute__((preserve_access_index)), apply_to = record)
struct task_cputime { __u64 utime; __u64 stime; __u64 sum_exec_runtime; };
struct cgroup_base_stat { struct task_cputime cputime; };
struct cgroup_rstat_cpu { struct cgroup_base_stat subtree_bstat; };
struct cgroup_rstat_base_cpu { struct cgroup_base_stat subtree_bstat; };
struct kernfs_node { __u64 id; };
struct cgroup;
struct cgroup_subsys_state { struct cgroup *cgroup; struct cgroup_subsys_state *parent; };
struct cgroup { struct cgroup_subsys_state self; struct kernfs_node *kn; struct cgroup_rstat_cpu *rstat_cpu; struct cgroup_rstat_base_cpu *rstat_base_cpu; int level; };
struct kernel_cpustat { __u64 cpustat[10]; };
struct rq { int online; };
struct seq_file;
struct bpf_iter_meta { struct seq_file *seq; __u64 session_id; __u64 seq_num; };
struct bpf_iter__cgroup { struct bpf_iter_meta *meta; struct cgroup *cgroup; };
#pragma clang attribute pop
extern void cgroup_rstat_flush(struct cgroup *cgrp) __ksym __weak;
extern void css_rstat_flush(struct cgroup_subsys_state *css) __ksym __weak;
extern const void __per_cpu_offset __ksym;
extern const struct kernel_cpustat kernel_cpustat __ksym;
extern const struct rq runqueues __ksym;
enum cpu_usage_stat { CPUTIME_USER, CPUTIME_NICE, CPUTIME_SYSTEM, CPUTIME_SOFTIRQ, CPUTIME_IRQ };
struct record { __u64 id, parent_id; __u32 cpu, flags; __u64 runtime_ns, user_ns, system_ns; };
struct control { __u32 capacity, cpus, count, overflow; };
struct { __uint(type, BPF_MAP_TYPE_ARRAY); __uint(max_entries, 1); __type(key, __u32); __type(value, struct control); } control SEC(".maps");
struct { __uint(type, BPF_MAP_TYPE_ARRAY); __uint(max_entries, 1); __type(key, __u32); __type(value, struct record); } records SEC(".maps");

static __noinline int read_new(struct cgroup *cg, unsigned long offset, struct record *r) {
    struct cgroup_rstat_base_cpu *base = BPF_CORE_READ(cg, rstat_base_cpu);
    if (!base) return -1;
    struct cgroup_rstat_base_cpu *pc = (void *)((unsigned long)base + offset);
    return BPF_CORE_READ_INTO(&r->runtime_ns, pc, subtree_bstat.cputime.sum_exec_runtime)
        || BPF_CORE_READ_INTO(&r->user_ns, pc, subtree_bstat.cputime.utime)
        || BPF_CORE_READ_INTO(&r->system_ns, pc, subtree_bstat.cputime.stime);
}

static __noinline int read_old(struct cgroup *cg, unsigned long offset, struct record *r) {
    struct cgroup_rstat_cpu *base = BPF_CORE_READ(cg, rstat_cpu);
    if (!base) return -1;
    struct cgroup_rstat_cpu *pc = (void *)((unsigned long)base + offset);
    return BPF_CORE_READ_INTO(&r->runtime_ns, pc, subtree_bstat.cputime.sum_exec_runtime)
        || BPF_CORE_READ_INTO(&r->user_ns, pc, subtree_bstat.cputime.utime)
        || BPF_CORE_READ_INTO(&r->system_ns, pc, subtree_bstat.cputime.stime);
}

static __noinline int online_cpu(__u32 cpu) {
    struct rq *rq = bpf_per_cpu_ptr(&runqueues, cpu);
    int online = 0;
    if (!rq || BPF_CORE_READ_INTO(&online, rq, online)) return -1;
    return online != 0;
}

static __noinline int read_root(__u32 cpu, struct record *r) {
    struct kernel_cpustat *stat = bpf_per_cpu_ptr(&kernel_cpustat, cpu);
    if (!stat) return -1;
    __u64 nice = 0, irq = 0, softirq = 0;
    if (BPF_CORE_READ_INTO(&r->user_ns, stat, cpustat[bpf_core_enum_value(enum cpu_usage_stat, CPUTIME_USER)])
        || BPF_CORE_READ_INTO(&nice, stat, cpustat[bpf_core_enum_value(enum cpu_usage_stat, CPUTIME_NICE)])
        || BPF_CORE_READ_INTO(&r->system_ns, stat, cpustat[bpf_core_enum_value(enum cpu_usage_stat, CPUTIME_SYSTEM)])
        || BPF_CORE_READ_INTO(&irq, stat, cpustat[bpf_core_enum_value(enum cpu_usage_stat, CPUTIME_IRQ)])
        || BPF_CORE_READ_INTO(&softirq, stat, cpustat[bpf_core_enum_value(enum cpu_usage_stat, CPUTIME_SOFTIRQ)])) return -1;
    // Match Linux root_cgroup_cputime: guest time is already included in user/nice.
    r->user_ns += nice;
    r->system_ns += irq + softirq;
    r->runtime_ns = r->user_ns + r->system_ns;
    return 0;
}

SEC("iter.s/cgroup")
int collect(struct bpf_iter__cgroup *ctx) {
    struct cgroup *cg = ctx->cgroup;
    if (!cg) return 0;
    __u32 zero = 0;
    struct control *ctl = bpf_map_lookup_elem(&control, &zero);
    if (!ctl) return 0;
    if (ctx->meta->seq_num == 0) {
        if (bpf_ksym_exists(css_rstat_flush)) css_rstat_flush(&cg->self);
        else if (bpf_ksym_exists(cgroup_rstat_flush)) cgroup_rstat_flush(cg);
        else return -95; // EOPNOTSUPP; never publish unflushed values as valid.
    }
    __u64 id = BPF_CORE_READ(cg, kn, id);
    struct cgroup_subsys_state *parent = BPF_CORE_READ(cg, self.parent);
    __u64 parent_id = parent ? BPF_CORE_READ(parent, cgroup, kn, id) : 0;
    int root = BPF_CORE_READ(cg, level) == 0;
    // Hard bound guards verifier complexity and configuration mistakes.
    int cpu;
    bpf_for(cpu, 0, 4096) {
        if (cpu >= ctl->cpus) break;
        int online = root ? online_cpu(cpu) : 1;
        if (root && online == 0) continue;
        if (ctl->count >= ctl->capacity) { ctl->overflow++; continue; }
        __u32 index = ctl->count++;
        struct record *r = bpf_map_lookup_elem(&records, &index);
        if (!r) { ctl->overflow++; continue; }
        __builtin_memset(r, 0, sizeof(*r));
        r->id = id; r->parent_id = parent_id; r->cpu = cpu;
        if (root) {
            r->flags = 2;
            if (online < 0 || read_root(cpu, r)) r->flags |= 4;
            else r->flags |= 1 | 8;
            continue;
        }
        // rstat pointers lack a percpu BTF tag on supported kernels, so the
        // typed bpf_per_cpu_ptr helper rejects them. Use the kernel's native
        // per-CPU offset table and checked probe reads instead.
        unsigned long offset = 0;
        if (bpf_probe_read_kernel(&offset, sizeof(offset), (const char *)&__per_cpu_offset + cpu * sizeof(offset))) {
            r->flags = 4; continue;
        }
        // Relocate each scalar field separately: CO-RE does not rearrange the
        // members of a struct copied wholesale from kernel memory.
        int err;
        if (bpf_core_field_exists(cg->rstat_base_cpu)) err = read_new(cg, offset, r);
        else err = read_old(cg, offset, r);
        if (err) { r->flags = 4; continue; }
        // Native inclusive runtime; user/system are RAW, before cpu.stat adjustment.
        r->flags = 1 | 8;
    }
    return 0;
}
char LICENSE[] SEC("license") = "GPL";
