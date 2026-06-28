export type UnknownRecord = Record<string, any>;

export type DetailField = {
  label: string;
  value: string;
};

export type RuntimeDetail = {
  title: string;
  eyebrow: string;
  copy: string;
  fields: DetailField[];
};

export type Filterable = {
  filterTags?: string[];
};

export type RuntimeEntry = Filterable & {
  id: string;
  label: string;
  status: any;
  deployment: any;
  type: string;
  authority: any;
  note: string;
  badge: any;
  detail: RuntimeDetail;
};

export type MeshItem = Filterable & {
  title: any;
  id?: any;
  meta?: any;
  copy?: string;
  detail?: RuntimeDetail;
};

export type MeshPanel = Filterable & {
  label: string;
  title: string;
  pills: DetailField[];
  items: MeshItem[];
  wide?: boolean;
  detail: RuntimeDetail;
};

export type TopologyItem = Filterable & {
  title: any;
  id?: any;
  copy?: string;
  meta: DetailField[];
  detail?: RuntimeDetail;
};

export type TopologySection = Filterable & {
  eyebrow: string;
  title: string;
  copy: string;
  stats: DetailField[];
  items: TopologyItem[];
};

export type RuntimeFilter = {
  value: string;
  label: string;
};

export type RuntimeSelection = {
  key: string;
  detail: RuntimeDetail;
};

export type RuntimeStatusModel = {
  summary: string;
  runtimes: RuntimeEntry[];
  meshPanels: MeshPanel[];
  topology: TopologySection[];
  filters: RuntimeFilter[];
  selectedFilter: string;
  detailSelection: RuntimeSelection | null;
};
