#ifndef LD_NAT_COMMON_H
#define LD_NAT_COMMON_H
#include "vmlinux.h"
#include "landscape_log.h"

#define GRESS_MASK (1 << 0)

#define COPY_ADDR_FROM(t, s) (__builtin_memcpy((t), (s), sizeof(t)))

static __always_inline int bpf_write_port(struct __sk_buff *skb, int port_off, __be16 to_port) {
    return bpf_skb_store_bytes(skb, port_off, &to_port, sizeof(to_port), 0);
}

static __always_inline int bpf_write_inet_addr(struct __sk_buff *skb, bool is_ipv4, int addr_off,
                                               union u_inet_addr *to_addr) {
    return bpf_skb_store_bytes(skb, addr_off, is_ipv4 ? &to_addr->ip : to_addr->all,
                               is_ipv4 ? sizeof(to_addr->ip) : sizeof(to_addr->all), 0);
}

static __always_inline int is_handle_protocol(const u8 protocol) {
    // TODO mDNS
    if (protocol == IPPROTO_TCP || protocol == IPPROTO_UDP || protocol == IPPROTO_ICMP ||
        protocol == NEXTHDR_ICMP) {
        return TC_ACT_OK;
    } else {
        return TC_ACT_UNSPEC;
    }
}

enum {
    NAT_MAPPING_INGRESS = 0,
    NAT_MAPPING_EGRESS = 1,
};

/// 作为 发出 和 接收 数据包时查询的 key
struct nat_mapping_key {
    u8 gress;
    u8 l4proto;
    __be16 from_port;
    union u_inet_addr from_addr;
};

struct nat_mapping_value {
    union u_inet_addr addr;
    // TODO： 触发这个关系的 ip 或者端口
    // 单独一张检查表， 使用这个 ip 获取是否需要检查
    union u_inet_addr trigger_addr;
    __be16 port;
    __be16 trigger_port;
    u8 is_static;
    u8 is_allow_reuse;
    u8 _pad[2];
    // 增加一个最后活跃时间
    u64 active_time;
    //
};

/// 作为静态映射 map
/// TODO: 支持多 Nat 网卡进行映射
struct nat_static_mapping_key {
    // 匹配数据长度
    __u32 prefixlen;
    u8 gress;
    u8 l4proto;
    __be16 from_port;
    union u_inet_addr from_addr;
};

//
struct nat_timer_key {
    u8 l4proto;
    u8 _pad[3];
    // Ac:Pc_An:Pn
    struct inet_pair pair_ip;
};

//
struct nat_timer_value {
    // 只关注 Timer 的状态
    u64 status;
    struct bpf_timer timer;
    // As
    union u_inet_addr trigger_saddr;
    // Ps
    u16 trigger_port;
    u8 gress;
    u8 _pad;
};

// 所能映射的范围
struct mapping_range {
    u16 start;
    u16 end;
};

// 用于搜寻可用的端口
struct search_port_ctx {
    struct nat_mapping_key ingress_key;
    struct mapping_range range;
    u16 remaining_size;
    // 小端序的端口
    u16 curr_port;
    bool found;
    u64 timeout_interval;
};

#endif /* LD_NAT_COMMON_H */
