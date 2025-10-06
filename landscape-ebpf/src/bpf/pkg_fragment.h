#ifndef __LD_PACKET_FRAGMENT_H__
#define __LD_PACKET_FRAGMENT_H__

#include "vmlinux.h"

#include <bpf/bpf_endian.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_core_read.h>

#include "pkg_def.h"
#include "pkg_scanner.h"

#define FRAG_CACHE_SIZE 1024 * 32
struct {
    __uint(type, BPF_MAP_TYPE_LRU_HASH);
    __type(key, struct fragment_cache_key);
    __type(value, struct fragment_cache_value);
    __uint(max_entries, FRAG_CACHE_SIZE);
} fragment_cache SEC(".maps");

static __always_inline int frag_info_track(const struct packet_offset_info *offset,
                                           struct inet_pair *ip_pair) {
#define BPF_LOG_TOPIC "frag_info_track"

    // 没有被分片的数据包, 无需进行记录
    if (offset->fragment_type == LD_NOT_F ||
        (offset->fragment_type == LD_END_F && offset->fragment_off == 0)) {
        return TC_ACT_OK;
    }

    if (is_icmp_error_pkt(offset)) {
        return TC_ACT_SHOT;
    }

    int ret;
    struct fragment_cache_key key = {0};
    key.l4proto = offset->l4_protocol;
    key.id = offset->fragment_id;

    COPY_ADDR_FROM(key.saddr.all, ip_pair->src_addr.all);
    COPY_ADDR_FROM(key.daddr.all, ip_pair->dst_addr.all);

    struct fragment_cache_value *value;
    if (offset->fragment_type == LD_MORE_F && offset->fragment_off == 0) {
        struct fragment_cache_value value_new;
        value_new.dport = ip_pair->dst_port;
        value_new.sport = ip_pair->src_port;

        ret = bpf_map_update_elem(&fragment_cache, &key, &value_new, BPF_ANY);
        if (ret) {
            return TC_ACT_SHOT;
        }
        value = (struct fragment_cache_value *)bpf_map_lookup_elem(&fragment_cache, &key);
        if (!value) {
            return TC_ACT_SHOT;
        }
    } else {
        value = (struct fragment_cache_value *)bpf_map_lookup_elem(&fragment_cache, &key);
        if (!value) {
            bpf_log_warn("fragmentation session of this packet was not tracked");
            return TC_ACT_SHOT;
        }
        ip_pair->src_port = value->sport;
        ip_pair->dst_port = value->dport;
    }

    return TC_ACT_OK;
#undef BPF_LOG_TOPIC
}
#endif /* __LD_PACKET_FRAGMENT_H__ */