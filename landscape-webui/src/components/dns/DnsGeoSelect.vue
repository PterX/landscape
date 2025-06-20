<script setup lang="ts">
import {
  delete_geo_site_config,
  get_geo_site_configs,
  search_geo_site_cache,
} from "@/api/geo/site";
import { GeoConfigKey } from "@/rust_bindings/common/geo";
import {
  GeoDomainConfig,
  GeoSiteSourceConfig,
} from "@/rust_bindings/common/geo_site";
import { computed, onMounted, ref } from "vue";

const key = defineModel<string>("geo_key", {
  required: true,
});
const name = defineModel<string>("geo_name", {
  required: true,
});
const inverse = defineModel<boolean>("geo_inverse", {
  default: false,
});

const emit = defineEmits(["refresh"]);

interface Prop {
  geo_site: GeoDomainConfig;
}
const props = defineProps<Prop>();
const loading_name = ref(false);
const loading_key = ref(false);

onMounted(async () => {
  await typing_name_key("");
  await typing_key("");
});

const configs = ref<GeoSiteSourceConfig[]>();
async function typing_name_key(query?: string) {
  try {
    loading_name.value = true;
    configs.value = await get_geo_site_configs(query);
  } finally {
    loading_name.value = false;
  }
}
const geo_name_options = computed(() => {
  let result = [];
  if (configs.value) {
    for (const config of configs.value) {
      result.push({
        label: config.name,
        value: config.name,
      });
    }
  }
  return result;
});

async function typing_key(query: string) {
  try {
    loading_key.value = true;
    keys.value = await search_geo_site_cache({
      name: name.value,
      key: query,
    });
  } finally {
    loading_key.value = false;
  }
}

const keys = ref<GeoConfigKey[]>();
const geo_key_options = computed(() => {
  let result = [];
  if (keys.value) {
    for (const each_key of keys.value) {
      result.push({
        label: `${each_key.name}-${each_key.key}`,
        value: each_key.key,
        data: each_key,
      });
    }
  }
  return result;
});

function select_key(value: string, option: any) {
  key.value = value;
  name.value = option.data.name;
}
</script>
<template>
  <n-flex :wrap="false" align="center">
    <n-popover trigger="hover">
      <template #trigger>
        <n-checkbox v-model:checked="inverse"> </n-checkbox>
      </template>
      <span>反选 </span>
    </n-popover>
    <n-input-group>
      <n-select
        :style="{ width: '33%' }"
        v-model:value="name"
        filterable
        placeholder="选择 geo 名称"
        :options="geo_name_options"
        :loading="loading_name"
        clearable
        remote
        @update:value="emit('refresh')"
        @search="typing_name_key"
      />
      <n-select
        v-model:value="key"
        filterable
        placeholder="筛选key"
        :options="geo_key_options"
        :loading="loading_key"
        clearable
        remote
        @update:value="select_key"
        @search="typing_key"
      />
    </n-input-group>
  </n-flex>
</template>
