import type { PropertyType, SensitivityLevel } from '@/types/template';

export type SampleTemplateLocale = 'zh' | 'en';

export interface SampleTemplateProperty {
  id: string;
  name: string;
  type: PropertyType;
  sensitivityLevel: SensitivityLevel;
  required?: boolean;
  options?: string[];
  /** 插件合约字段映射 — 当此属性映射到插件合约中的字段时为 true。 */
  contractField?: boolean;
}

export interface SampleTemplate {
  key: string;
  locale: SampleTemplateLocale;
  category: 'identity' | 'travel' | 'financial' | 'professional';
  icon: string;
  name: string;
  properties: SampleTemplateProperty[];
  /** 插件合约类型 ID — 绑定到插件合约的模板类型标识。 */
  contractTypeId?: string;
}

export const SAMPLE_TEMPLATES_ZH: SampleTemplate[] = [
  {
    key: 'zh_identity',
    locale: 'zh',
    category: 'identity',
    icon: 'user',
    name: '身份信息',
    properties: [
      { id: 'fullName', name: '姓名', type: 'text', sensitivityLevel: 'public', required: true },
      { id: 'dateOfBirth', name: '出生日期', type: 'date', sensitivityLevel: 'internal' },
      { id: 'nationality', name: '国籍', type: 'text', sensitivityLevel: 'public' },
      { id: 'idNumber', name: '证件号码', type: 'text', sensitivityLevel: 'critical' },
      { id: 'email', name: '电子邮箱', type: 'email', sensitivityLevel: 'internal' },
      { id: 'phone', name: '电话', type: 'phone', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'zh_id_card',
    locale: 'zh',
    category: 'identity',
    icon: 'id_card',
    name: '身份证',
    properties: [
      { id: 'fullName', name: '姓名', type: 'text', sensitivityLevel: 'public', required: true },
      { id: 'idNumber', name: '身份证号', type: 'text', sensitivityLevel: 'critical' },
      { id: 'dateOfBirth', name: '出生日期', type: 'date', sensitivityLevel: 'internal' },
      { id: 'nationality', name: '国籍', type: 'text', sensitivityLevel: 'public' },
      { id: 'issueDate', name: '签发日期', type: 'date', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'zh_passport',
    locale: 'zh',
    category: 'travel',
    icon: 'bookmarked',
    name: '护照',
    properties: [
      { id: 'fullName', name: '姓名', type: 'text', sensitivityLevel: 'public', required: true },
      {
        id: 'passportNumber',
        name: '护照号码',
        type: 'text',
        sensitivityLevel: 'critical',
        required: true,
      },
      { id: 'nationality', name: '国籍', type: 'text', sensitivityLevel: 'public' },
      { id: 'dateOfBirth', name: '出生日期', type: 'date', sensitivityLevel: 'internal' },
      { id: 'issueDate', name: '签发日期', type: 'date', sensitivityLevel: 'internal' },
      { id: 'expiryDate', name: '有效期至', type: 'date', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'zh_visa',
    locale: 'zh',
    category: 'travel',
    icon: 'ticket',
    name: '签证',
    properties: [
      { id: 'country', name: '国家', type: 'text', sensitivityLevel: 'public' },
      { id: 'visaType', name: '签证类型', type: 'text', sensitivityLevel: 'public' },
      { id: 'number', name: '签证号码', type: 'text', sensitivityLevel: 'critical' },
      { id: 'issueDate', name: '签发日期', type: 'date', sensitivityLevel: 'internal' },
      { id: 'expiryDate', name: '有效期至', type: 'date', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'zh_bank_account',
    locale: 'zh',
    category: 'financial',
    icon: 'landmark',
    name: '银行账户',
    properties: [
      { id: 'bankName', name: '银行名称', type: 'text', sensitivityLevel: 'public' },
      { id: 'accountNumber', name: '账号', type: 'text', sensitivityLevel: 'critical' },
      { id: 'accountType', name: '账户类型', type: 'text', sensitivityLevel: 'public' },
      { id: 'currency', name: '币种', type: 'text', sensitivityLevel: 'public' },
    ],
  },
  {
    key: 'zh_bank_card',
    locale: 'zh',
    category: 'financial',
    icon: 'credit_card',
    name: '银行卡',
    properties: [
      { id: 'cardNumber', name: '卡号', type: 'text', sensitivityLevel: 'critical' },
      { id: 'cardType', name: '卡类型', type: 'text', sensitivityLevel: 'public' },
      { id: 'holderName', name: '持卡人', type: 'text', sensitivityLevel: 'public' },
      { id: 'expiryDate', name: '有效期至', type: 'date', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'zh_education',
    locale: 'zh',
    category: 'professional',
    icon: 'graduation',
    name: '教育经历',
    properties: [
      { id: 'institution', name: '院校', type: 'text', sensitivityLevel: 'public' },
      { id: 'degree', name: '学位', type: 'text', sensitivityLevel: 'public' },
      { id: 'field', name: '专业', type: 'text', sensitivityLevel: 'public' },
      { id: 'startDate', name: '开始日期', type: 'date', sensitivityLevel: 'public' },
      { id: 'endDate', name: '结束日期', type: 'date', sensitivityLevel: 'public' },
    ],
  },
  {
    key: 'zh_employment',
    locale: 'zh',
    category: 'professional',
    icon: 'briefcase',
    name: '工作经历',
    properties: [
      { id: 'company', name: '公司', type: 'text', sensitivityLevel: 'public' },
      { id: 'position', name: '职位', type: 'text', sensitivityLevel: 'public' },
      { id: 'startDate', name: '开始日期', type: 'date', sensitivityLevel: 'public' },
      { id: 'endDate', name: '结束日期', type: 'date', sensitivityLevel: 'public' },
    ],
  },
  {
    key: 'zh_address',
    locale: 'zh',
    category: 'identity',
    icon: 'home',
    name: '地址',
    contractTypeId: 'com.solosoul.official.address-fmt/v1',
    properties: [
      { id: 'street', name: '具体街道地址', type: 'text', sensitivityLevel: 'sensitive', contractField: true },
      { id: 'district', name: '县/区', type: 'text', sensitivityLevel: 'internal', contractField: true },
      { id: 'city', name: '城市', type: 'text', sensitivityLevel: 'public', contractField: true },
      { id: 'province', name: '省/自治区/直辖市/特别行政区', type: 'text', sensitivityLevel: 'public', contractField: true },
      { id: 'country', name: '国家', type: 'text', sensitivityLevel: 'public', contractField: true },
      { id: 'postalCode', name: '邮编', type: 'text', sensitivityLevel: 'internal', contractField: true },
    ],
  },
];

export const SAMPLE_TEMPLATES_EN: SampleTemplate[] = [
  {
    key: 'en_identity',
    locale: 'en',
    category: 'identity',
    icon: 'user',
    name: 'Identity',
    properties: [
      {
        id: 'fullName',
        name: 'Full Name',
        type: 'text',
        sensitivityLevel: 'public',
        required: true,
      },
      { id: 'dateOfBirth', name: 'Date of Birth', type: 'date', sensitivityLevel: 'internal' },
      { id: 'nationality', name: 'Nationality', type: 'text', sensitivityLevel: 'public' },
      { id: 'idNumber', name: 'ID Number', type: 'text', sensitivityLevel: 'critical' },
      { id: 'email', name: 'Email', type: 'email', sensitivityLevel: 'internal' },
      { id: 'phone', name: 'Phone', type: 'phone', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'en_id_card',
    locale: 'en',
    category: 'identity',
    icon: 'id_card',
    name: 'ID Card',
    properties: [
      {
        id: 'fullName',
        name: 'Full Name',
        type: 'text',
        sensitivityLevel: 'public',
        required: true,
      },
      { id: 'idNumber', name: 'ID Number', type: 'text', sensitivityLevel: 'critical' },
      { id: 'dateOfBirth', name: 'Date of Birth', type: 'date', sensitivityLevel: 'internal' },
      { id: 'nationality', name: 'Nationality', type: 'text', sensitivityLevel: 'public' },
      { id: 'issueDate', name: 'Issue Date', type: 'date', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'en_passport',
    locale: 'en',
    category: 'travel',
    icon: 'bookmarked',
    name: 'Passport',
    properties: [
      {
        id: 'fullName',
        name: 'Full Name',
        type: 'text',
        sensitivityLevel: 'public',
        required: true,
      },
      {
        id: 'passportNumber',
        name: 'Passport Number',
        type: 'text',
        sensitivityLevel: 'critical',
        required: true,
      },
      { id: 'nationality', name: 'Nationality', type: 'text', sensitivityLevel: 'public' },
      { id: 'dateOfBirth', name: 'Date of Birth', type: 'date', sensitivityLevel: 'internal' },
      { id: 'issueDate', name: 'Issue Date', type: 'date', sensitivityLevel: 'internal' },
      { id: 'expiryDate', name: 'Expiry Date', type: 'date', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'en_visa',
    locale: 'en',
    category: 'travel',
    icon: 'ticket',
    name: 'Visa',
    properties: [
      { id: 'country', name: 'Country', type: 'text', sensitivityLevel: 'public' },
      { id: 'visaType', name: 'Visa Type', type: 'text', sensitivityLevel: 'public' },
      { id: 'number', name: 'Number', type: 'text', sensitivityLevel: 'critical' },
      { id: 'issueDate', name: 'Issue Date', type: 'date', sensitivityLevel: 'internal' },
      { id: 'expiryDate', name: 'Expiry Date', type: 'date', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'en_bank_account',
    locale: 'en',
    category: 'financial',
    icon: 'landmark',
    name: 'Bank Account',
    properties: [
      { id: 'bankName', name: 'Bank Name', type: 'text', sensitivityLevel: 'public' },
      { id: 'accountNumber', name: 'Account Number', type: 'text', sensitivityLevel: 'critical' },
      { id: 'accountType', name: 'Account Type', type: 'text', sensitivityLevel: 'public' },
      { id: 'currency', name: 'Currency', type: 'text', sensitivityLevel: 'public' },
    ],
  },
  {
    key: 'en_credit_card',
    locale: 'en',
    category: 'financial',
    icon: 'credit_card',
    name: 'Credit Card',
    properties: [
      { id: 'cardNumber', name: 'Card Number', type: 'text', sensitivityLevel: 'critical' },
      { id: 'cardType', name: 'Card Type', type: 'text', sensitivityLevel: 'public' },
      { id: 'holderName', name: 'Holder Name', type: 'text', sensitivityLevel: 'public' },
      { id: 'expiryDate', name: 'Expiry Date', type: 'date', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'en_education',
    locale: 'en',
    category: 'professional',
    icon: 'graduation',
    name: 'Education',
    properties: [
      { id: 'institution', name: 'Institution', type: 'text', sensitivityLevel: 'public' },
      { id: 'degree', name: 'Degree', type: 'text', sensitivityLevel: 'public' },
      { id: 'field', name: 'Field of Study', type: 'text', sensitivityLevel: 'public' },
      { id: 'startDate', name: 'Start Date', type: 'date', sensitivityLevel: 'public' },
      { id: 'endDate', name: 'End Date', type: 'date', sensitivityLevel: 'public' },
    ],
  },
  {
    key: 'en_employment',
    locale: 'en',
    category: 'professional',
    icon: 'briefcase',
    name: 'Employment',
    properties: [
      { id: 'company', name: 'Company', type: 'text', sensitivityLevel: 'public' },
      { id: 'position', name: 'Position', type: 'text', sensitivityLevel: 'public' },
      { id: 'startDate', name: 'Start Date', type: 'date', sensitivityLevel: 'public' },
      { id: 'endDate', name: 'End Date', type: 'date', sensitivityLevel: 'public' },
    ],
  },
  {
    key: 'en_address',
    locale: 'en',
    category: 'identity',
    icon: 'home',
    name: 'Address',
    contractTypeId: 'com.solosoul.official.address-fmt/v1',
    properties: [
      { id: 'street', name: 'Street address', type: 'text', sensitivityLevel: 'sensitive', contractField: true },
      { id: 'district', name: 'District / County', type: 'text', sensitivityLevel: 'internal', contractField: true },
      { id: 'city', name: 'City', type: 'text', sensitivityLevel: 'public', contractField: true },
      { id: 'province', name: 'Province / State', type: 'text', sensitivityLevel: 'public', contractField: true },
      { id: 'country', name: 'Country', type: 'text', sensitivityLevel: 'public', contractField: true },
      { id: 'postalCode', name: 'Postal code', type: 'text', sensitivityLevel: 'internal', contractField: true },
    ],
  },
];

export const SAMPLE_TEMPLATES_BY_LOCALE: Record<SampleTemplateLocale, SampleTemplate[]> = {
  zh: SAMPLE_TEMPLATES_ZH,
  en: SAMPLE_TEMPLATES_EN,
};

/** 全部示例模板，仅在需要一次性遍历全部场景时使用；UI 默认按 locale 分组展示。 */
export const SAMPLE_TEMPLATES: SampleTemplate[] = [...SAMPLE_TEMPLATES_ZH, ...SAMPLE_TEMPLATES_EN];

export function getDefaultLocaleTab(language?: string): SampleTemplateLocale {
  if (language && language.toLowerCase().startsWith('zh')) return 'zh';
  return 'en';
}
