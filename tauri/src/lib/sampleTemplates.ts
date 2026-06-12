import type { PropertyType, SensitivityLevel } from '@/types/template';

export interface SampleTemplateProperty {
  id: string;
  nameI18nKey: string;
  nameFallback: string;
  type: PropertyType;
  sensitivityLevel: SensitivityLevel;
  required?: boolean;
  options?: string[];
}

export interface SampleTemplate {
  key: string;
  category: 'identity' | 'travel' | 'financial' | 'professional';
  icon: string;
  nameI18nKey: string;
  nameFallback: string;
  properties: SampleTemplateProperty[];
}

export const SAMPLE_TEMPLATES: SampleTemplate[] = [
  {
    key: 'identity',
    category: 'identity',
    icon: 'identity',
    nameI18nKey: 'editor:templates.identity',
    nameFallback: '身份信息',
    properties: [
      { id: 'fullName', nameI18nKey: 'editor:fields.fullName', nameFallback: '姓名', type: 'text', sensitivityLevel: 'public', required: true },
      { id: 'dateOfBirth', nameI18nKey: 'editor:fields.dateOfBirth', nameFallback: '出生日期', type: 'date', sensitivityLevel: 'internal' },
      { id: 'nationality', nameI18nKey: 'editor:fields.nationality', nameFallback: '国籍', type: 'text', sensitivityLevel: 'public' },
      { id: 'idNumber', nameI18nKey: 'editor:fields.idNumber', nameFallback: '证件号码', type: 'text', sensitivityLevel: 'critical' },
      { id: 'email', nameI18nKey: 'editor:fields.email', nameFallback: '电子邮箱', type: 'email', sensitivityLevel: 'internal' },
      { id: 'phone', nameI18nKey: 'editor:fields.phone', nameFallback: '电话', type: 'phone', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'idCard',
    category: 'identity',
    icon: 'idCard',
    nameI18nKey: 'editor:templates.idCard',
    nameFallback: '身份证',
    properties: [
      { id: 'fullName', nameI18nKey: 'editor:fields.fullName', nameFallback: '姓名', type: 'text', sensitivityLevel: 'public', required: true },
      { id: 'idNumber', nameI18nKey: 'editor:fields.idNumber', nameFallback: '身份证号', type: 'text', sensitivityLevel: 'critical' },
      { id: 'dateOfBirth', nameI18nKey: 'editor:fields.dateOfBirth', nameFallback: '出生日期', type: 'date', sensitivityLevel: 'internal' },
      { id: 'nationality', nameI18nKey: 'editor:fields.nationality', nameFallback: '国籍', type: 'text', sensitivityLevel: 'public' },
      { id: 'issueDate', nameI18nKey: 'editor:fields.issueDate', nameFallback: '签发日期', type: 'date', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'passport',
    category: 'travel',
    icon: 'passport',
    nameI18nKey: 'editor:templates.passport',
    nameFallback: '护照',
    properties: [
      { id: 'fullName', nameI18nKey: 'editor:fields.fullName', nameFallback: '姓名', type: 'text', sensitivityLevel: 'public', required: true },
      { id: 'passportNumber', nameI18nKey: 'editor:fields.passportNumber', nameFallback: '护照号码', type: 'text', sensitivityLevel: 'critical', required: true },
      { id: 'nationality', nameI18nKey: 'editor:fields.nationality', nameFallback: '国籍', type: 'text', sensitivityLevel: 'public' },
      { id: 'dateOfBirth', nameI18nKey: 'editor:fields.dateOfBirth', nameFallback: '出生日期', type: 'date', sensitivityLevel: 'internal' },
      { id: 'issueDate', nameI18nKey: 'editor:fields.issueDate', nameFallback: '签发日期', type: 'date', sensitivityLevel: 'internal' },
      { id: 'expiryDate', nameI18nKey: 'editor:fields.expiryDate', nameFallback: '有效期至', type: 'date', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'visa',
    category: 'travel',
    icon: 'visa',
    nameI18nKey: 'editor:templates.visa',
    nameFallback: '签证',
    properties: [
      { id: 'country', nameI18nKey: 'editor:fields.country', nameFallback: '国家', type: 'text', sensitivityLevel: 'public' },
      { id: 'visaType', nameI18nKey: 'editor:fields.visaType', nameFallback: '签证类型', type: 'text', sensitivityLevel: 'public' },
      { id: 'number', nameI18nKey: 'editor:fields.number', nameFallback: '签证号码', type: 'text', sensitivityLevel: 'critical' },
      { id: 'issueDate', nameI18nKey: 'editor:fields.issueDate', nameFallback: '签发日期', type: 'date', sensitivityLevel: 'internal' },
      { id: 'expiryDate', nameI18nKey: 'editor:fields.expiryDate', nameFallback: '有效期至', type: 'date', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'bank',
    category: 'financial',
    icon: 'bank',
    nameI18nKey: 'editor:templates.bank',
    nameFallback: '银行账户',
    properties: [
      { id: 'bankName', nameI18nKey: 'editor:fields.bankName', nameFallback: '银行名称', type: 'text', sensitivityLevel: 'public' },
      { id: 'accountNumber', nameI18nKey: 'editor:fields.accountNumber', nameFallback: '账号', type: 'text', sensitivityLevel: 'critical' },
      { id: 'accountType', nameI18nKey: 'editor:fields.accountType', nameFallback: '账户类型', type: 'text', sensitivityLevel: 'public' },
      { id: 'currency', nameI18nKey: 'editor:fields.currency', nameFallback: '币种', type: 'text', sensitivityLevel: 'public' },
    ],
  },
  {
    key: 'card',
    category: 'financial',
    icon: 'card',
    nameI18nKey: 'editor:templates.card',
    nameFallback: '银行卡',
    properties: [
      { id: 'cardNumber', nameI18nKey: 'editor:fields.cardNumber', nameFallback: '卡号', type: 'text', sensitivityLevel: 'critical' },
      { id: 'cardType', nameI18nKey: 'editor:fields.cardType', nameFallback: '卡类型', type: 'text', sensitivityLevel: 'public' },
      { id: 'holderName', nameI18nKey: 'editor:fields.holderName', nameFallback: '持卡人', type: 'text', sensitivityLevel: 'public' },
      { id: 'expiryDate', nameI18nKey: 'editor:fields.expiryDate', nameFallback: '有效期至', type: 'date', sensitivityLevel: 'internal' },
    ],
  },
  {
    key: 'education',
    category: 'professional',
    icon: 'education',
    nameI18nKey: 'editor:templates.education',
    nameFallback: '教育经历',
    properties: [
      { id: 'institution', nameI18nKey: 'editor:fields.institution', nameFallback: '院校', type: 'text', sensitivityLevel: 'public' },
      { id: 'degree', nameI18nKey: 'editor:fields.degree', nameFallback: '学位', type: 'text', sensitivityLevel: 'public' },
      { id: 'field', nameI18nKey: 'editor:fields.field', nameFallback: '专业', type: 'text', sensitivityLevel: 'public' },
      { id: 'startDate', nameI18nKey: 'editor:fields.startDate', nameFallback: '开始日期', type: 'date', sensitivityLevel: 'public' },
      { id: 'endDate', nameI18nKey: 'editor:fields.endDate', nameFallback: '结束日期', type: 'date', sensitivityLevel: 'public' },
    ],
  },
  {
    key: 'employment',
    category: 'professional',
    icon: 'employment',
    nameI18nKey: 'editor:templates.employment',
    nameFallback: '工作经历',
    properties: [
      { id: 'company', nameI18nKey: 'editor:fields.company', nameFallback: '公司', type: 'text', sensitivityLevel: 'public' },
      { id: 'position', nameI18nKey: 'editor:fields.position', nameFallback: '职位', type: 'text', sensitivityLevel: 'public' },
      { id: 'startDate', nameI18nKey: 'editor:fields.startDate', nameFallback: '开始日期', type: 'date', sensitivityLevel: 'public' },
      { id: 'endDate', nameI18nKey: 'editor:fields.endDate', nameFallback: '结束日期', type: 'date', sensitivityLevel: 'public' },
    ],
  },
];
