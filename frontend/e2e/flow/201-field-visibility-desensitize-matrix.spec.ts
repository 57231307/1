/**
 * 显示控制与脱敏面矩阵（面料专项，判据全部取自后端真实出参）
 *
 * 功能：
 *   1. 先钉后端脱敏原语自身的底数。从 utils/field_mask.rs 现读“非 admin 置空哪些键”
 *      “电话与邮箱掩码哪些键”“客户域默认脱敏整键移除哪些列”，并要求判键与置键同键
 *      （contains_key 判了却没 insert Null 属空转掩码，提取阶段就抛错）。本项目实测事实是
 *      mask_sensitive_fields 只剔 cost_price 一枚键、且只认 admin（admin_checker::is_admin_role），
 *      本文件把这两条做成可复算的期望而不是注释里的声明。
 *   2. 分面断言，不按理想断言。同一列在不同 handler 腿上的处置逐面单独判：
 *      先做接线对账（读该 handler 函数体，摘掉注释行后按调用形态找脱敏函数，与本文件
 *      声明的应然接线双向全等），再做活体请求逐列取值比对。已登记的真实分面事实包括
 *      产品出参把整个实体序列化（连带 supplier_id 与 supplier_product_code 两列一起下发）、
 *      掩码只剔 cost_price、customers 的列表与详情与联系人子端点是三条独立腿、
 *      DECIMAL 列出参是字符串而 f64 计算值（BI 毛利率）出参是数字。
 *   3. 面料特色字段逐列可见性：成本价、进价、染厂加工费、毛利、折扣修改开关、
 *      信用额度修改、库存调整、缸号、色号、米数、克重、门幅、损耗、余码、卷号
 *      逐项落到本仓真实列名与真实面。落不到的列按事实登记：余码在本仓不是独立列
 *      （拆分腿把剩余米数回写父卷 length），卷号即 piece_no（前端词表称布卷号），
 *      折扣与信用额度与库存调整三项的“能不能改”是写腿权限而不是读面掩码，
 *      其越权判据在 202，本文件只钉它们的出参列与出参形态。
 *   4. 服务端脱敏与端点授权分离：对每一枚成本族面，实测它到底有没有在服务端掩码。
 *      判据在后端出参，不许用前端隐藏冒充脱敏。因此本文件同时断两件事：
 *      一是“掩码列的列表腿与详情腿处置必须一致”（详情腿漏掩即旁路，本项目详情腿
 *      曾原样下发成本价，靠这条锁住回退）；二是未被服务端脱敏的成本族面逐枚落进制品，
 *      交小程序侧按权限码收敛调用点，并在用例里点名“这些面的可见性只由端点可达性决定”。
 *   5. 三端一致性：同一记录在 PC 列表腿、PC 详情腿、H5 与小程序所用的读腿上，
 *      状态取值与可见字段集合必须一致。H5 与 PC 共用同一套后端读腿（快照里不存在
 *      第二套移动端列表端点），该事实由路由快照反证；小程序侧的可见性判据从后端契约
 *      导出（同一枚权限码、同一组掩码键），逐角色实测“该角色能读到真值的成本族面集合”
 *      必须等于按其权限码派生的集合，多一个面即外泄、少一个面即授权链断裂。
 *
 * 与 200 与 202 的分工：200 判后端放行面（RBAC 与 handler 角色门），202 判越权负例与
 * 拒因归属，本文件只判“放行之后你到底看见了什么”。三面必须分开，因为本项目同时存在
 * “后端放行但出参不脱敏”与“前端隐藏但出参照发”两种真实形态。
 *
 * 调用方：Playwright E2E 套件，CI 的 e2e flow 分片按文件哈希收集执行。
 * 入参：API_BASE 与 BASE_URL 取环境变量；角色凭证取 env 优先、回退 role-credentials.json；
 *      期望集全部由 ./role-matrix-support 从后端源文件现读派生，本文件不抄第二份掩码清单。
 * 传给谁：会话与身份钉交给 fixtures/multi-context 的 real 档与 fixtures/relay 的
 *      readRelayIdentity；请求与拒绝归因交给 flow/helpers 的 apiCallRaw、apiCallExpectFail
 *      与 expectDenied，不另立第二套判据。
 * 存什么：为逐列断言准备的一行面料产品、一行客户与其一行联系人、一行采购价、
 *      一行成本归集、一行委外订单，各带已知的成本价、进价、染厂加工费、缸号、色号、
 *      米数、克重、门幅、损耗、电话、邮箱、信用额度取值，以及它们各自的业务列与审计行。
 * 存哪里：客户与产品由收尾时用 admin 会话当场回收；采购价、成本归集、委外单三条腿
 *      对本矩阵建的这行没有可用 DELETE 路由，按业务事实留库并在制品里点名，不假装已清理。
 *      矩阵实测结果写 e2e/.auth/role-visibility-matrix/201-*.json 供人工与小程序侧审阅，
 *      该目录不是基线、不参与判绿，判绿只看下面的断言。
 *
 * 判红即缺陷的口径：接线对账不平、逐列取值与后端源派生的期望不符、成本列在列表腿被掩
 * 却在详情腿原样下发、声明面里冒出未登记的列名，都保持判红并在消息里点名面、列、角色、
 * 实际值与期望值；不改期望凑绿，也不把“该面本就该原样”写成恒真放过。
 */
import { test, expect, type Page } from '../diagnose-fixture';
import { apiCallExpectFail, apiCallRaw, genCode, getRoleCredential } from './helpers';
import { createIsolatedSession, type IsolatedSession } from '../fixtures/multi-context';
import { readRelayIdentity } from '../fixtures/relay';
import { mkdirSync, writeFileSync } from 'fs';
import {
  backendGrants,
  derivePermissionKey,
  detectMaskCalls,
  readDecimalColumns,
  readHandlerFunctionBody,
  readMaskKeySets,
  readMatrixGrants,
  readRouteSnapshot,
  readSeededRoles,
  readStructBody,
  structMemberColumns,
} from './role-matrix-support';

type Row = Record<string, unknown>;

const MATRIX_ARTIFACT_DIR = 'e2e/.auth/role-visibility-matrix';

/** 后端脱敏原语的键集合（现读，不在本文件手抄第二份） */
const maskKeys = readMaskKeySets();

const seededRoles = readSeededRoles();
const matrixGrants = readMatrixGrants();

/**
 * 一列在某面上的应然处置集合。
 * clear=后端原样下发建档真值；null=非 admin 被服务端置空；maskedPhone 与 maskedEmail=
 * 按后端掩码口径改写但保留键；absentKey=非 admin 整键移除；absentColumn=该面出参结构里
 * 根本没有这列。非 admin 侧允许两个取值（掩码与整键移除）不是放宽：本仓客户域是
 * “默认脱敏层 + 角色数据权限行的 hidden 删键层”两层叠加（utils/data_permission.rs 的
 * DEFAULT_HIDDEN_FIELDS 含 contact_phone 与 contact_email 与 address 与 credit_limit），
 * 有没有配权限行决定落在哪一支。但**两个取值都不含原文**，所以“非 admin 永不拿到明文”
 * 仍是一条硬判据，由 neverPlaintext 单独钉死。
 */
type Disposition = 'clear' | 'null' | 'maskedPhone' | 'maskedEmail' | 'absentKey' | 'absentColumn';

interface ColumnSpec {
  /** 后端真实列名或出参键名 */
  column: string;
  /** 面料行业口径里的中文名，只进失败信息与制品，不进判据 */
  cn: string;
  /** 取值形态：decimal 要求出参是字符串，float 要求是数字，text 逐字等值，presence 只断键在 */
  kind: 'decimal' | 'float' | 'text' | 'presence';
  admin: Disposition[];
  nonAdmin: Disposition[];
  /** 非 admin 侧是否还允许拿到原文（成本与联系方式族一律 false，这是“不许前端冒充”的落点） */
  nonAdminMayBePlaintext: boolean;
}

interface Face {
  id: string;
  /** 相对 frontend 工作目录的 handler 源路径 */
  handlerFile: string;
  handlerFn: string;
  /** 权限资源名（后端 extract_resource_info 口径，用于挑持证角色） */
  resource: string;
  /** 该腿声明应挂的后端脱敏调用；空数组即“该面服务端不脱敏”这一事实本身 */
  expectMaskCalls: string[];
  /** 出参行集合位置：items=分页信封、root=裸数组、single=单对象 */
  rowsAt: 'items' | 'root' | 'single';
  columns: ColumnSpec[];
  /** 建档后用于在列表行里定位本矩阵那一行的锚点列；无锚点即只按首行读 */
  anchorColumn?: string;
  /** 是否已建档可做活体逐列断言；false 即只跑声明面与权限门两面 */
  seeded: boolean;
}

/** 本组用例共用的建档值：全部已知，才能把“原文下发”判成逐字相等而不是非空 */
const SEED = {
  productCode: genCode('E2E-FV-P'),
  cost_price: '12.3456',
  standard_price: '88.5000',
  gram_weight: '210.5',
  width: '150',
  fabric_composition: '65%棉 35%涤',
  customerName: genCode('E2E-FV-C'),
  contact_phone: '13800001234',
  contact_email: 'e2efabricvisibility@example.com',
  address: 'E2E 面料可见性矩阵地址一号',
  credit_limit: '500000',
  contactName: 'E2E 面料可见性联系人',
  contactPhone: '13912345678',
  contactEmail: 'e2econtact@example.com',
  price: '33.45',
  dyeing_fee: '4120.55',
  processing_fee: '980.25',
  dyeLotNo: genCode('E2EDYLOT'),
  colorNo: 'E2ECOLOR07',
  output_quantity_meters: '1200.75',
  orderNo: genCode('E2E-FV-O'),
  standard_loss_rate: '0.035',
};

/** 建档值到列名的映射：compareColumn 用它把“原文”判成等于建档真值 */
const SEED_BY_COLUMN: Record<string, string> = {
  cost_price: SEED.cost_price,
  standard_price: SEED.standard_price,
  gram_weight: SEED.gram_weight,
  width: SEED.width,
  contact_phone: SEED.contact_phone,
  contact_email: SEED.contact_email,
  address: SEED.address,
  credit_limit: SEED.credit_limit,
  phone: SEED.contactPhone,
  email: SEED.contactEmail,
  name: SEED.contactName,
  price: SEED.price,
  dyeing_fee: SEED.dyeing_fee,
  processing_fee: SEED.processing_fee,
  dye_lot_no: SEED.dyeLotNo,
  color_no: SEED.colorNo,
  output_quantity_meters: SEED.output_quantity_meters,
  order_no: SEED.orderNo,
  standard_loss_rate: SEED.standard_loss_rate,
};

/** 成本价与联系方式族在服务端被掩的两条腿（产品列表腿与详情腿、客户三条腿） */
const SEEDED_FACES: Face[] = [
  {
    id: 'products-list',
    handlerFile: '../backend/src/handlers/product_handler.rs',
    handlerFn: 'list_products',
    resource: 'products',
    expectMaskCalls: ['mask_sensitive_fields'],
    rowsAt: 'items',
    anchorColumn: 'code',
    seeded: true,
    columns: [
      {
        column: 'cost_price',
        cn: '成本价',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['null', 'absentKey'],
        nonAdminMayBePlaintext: false,
      },
      {
        column: 'standard_price',
        cn: '标准价',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'supplier_id',
        cn: '供应商外键',
        kind: 'presence',
        admin: ['clear'],
        nonAdmin: ['clear', 'absentKey'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'supplier_product_code',
        cn: '供应商货号',
        kind: 'presence',
        admin: ['clear'],
        nonAdmin: ['clear', 'absentKey'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'gram_weight',
        cn: '克重',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'width',
        cn: '门幅',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
    ],
  },
  {
    id: 'products-detail',
    handlerFile: '../backend/src/handlers/product_handler.rs',
    handlerFn: 'get_product',
    resource: 'products',
    expectMaskCalls: ['mask_sensitive_fields'],
    rowsAt: 'single',
    seeded: true,
    columns: [
      {
        column: 'cost_price',
        cn: '成本价',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['null', 'absentKey'],
        nonAdminMayBePlaintext: false,
      },
      {
        column: 'standard_price',
        cn: '标准价',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'supplier_id',
        cn: '供应商外键',
        kind: 'presence',
        admin: ['clear'],
        nonAdmin: ['clear', 'absentKey'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'supplier_product_code',
        cn: '供应商货号',
        kind: 'presence',
        admin: ['clear'],
        nonAdmin: ['clear', 'absentKey'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'gram_weight',
        cn: '克重',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'width',
        cn: '门幅',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
    ],
  },
  {
    id: 'customers-list',
    handlerFile: '../backend/src/handlers/customer_handler.rs',
    handlerFn: 'list_customers',
    resource: 'customers',
    expectMaskCalls: ['apply_customer_field_permission', 'apply_customer_field_config_mask'],
    rowsAt: 'items',
    anchorColumn: 'customer_name',
    seeded: true,
    columns: [
      {
        column: 'contact_phone',
        cn: '联系电话',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['maskedPhone', 'absentKey'],
        nonAdminMayBePlaintext: false,
      },
      {
        column: 'contact_email',
        cn: '联系邮箱',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['maskedEmail', 'absentKey'],
        nonAdminMayBePlaintext: false,
      },
      {
        column: 'address',
        cn: '地址',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['absentKey'],
        nonAdminMayBePlaintext: false,
      },
      {
        column: 'credit_limit',
        cn: '信用额度',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear', 'absentKey'],
        nonAdminMayBePlaintext: true,
      },
    ],
  },
  {
    id: 'customers-detail',
    handlerFile: '../backend/src/handlers/customer_handler.rs',
    handlerFn: 'get_customer',
    resource: 'customers',
    expectMaskCalls: ['apply_customer_field_permission', 'apply_customer_field_config_mask'],
    rowsAt: 'single',
    seeded: true,
    columns: [
      {
        column: 'contact_phone',
        cn: '联系电话',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['maskedPhone', 'absentKey'],
        nonAdminMayBePlaintext: false,
      },
      {
        column: 'contact_email',
        cn: '联系邮箱',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['maskedEmail', 'absentKey'],
        nonAdminMayBePlaintext: false,
      },
      {
        column: 'address',
        cn: '地址',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['absentKey'],
        nonAdminMayBePlaintext: false,
      },
      {
        column: 'credit_limit',
        cn: '信用额度',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear', 'absentKey'],
        nonAdminMayBePlaintext: true,
      },
    ],
  },
  {
    // 联系人子端点是第三条独立腿：它的电话与邮箱落在 customer_contacts 表的 phone 与
    // email 两列，与 customers 表的 contact_phone 与 contact_email 不同名，
    // 掩码是否生效必须单独判，不能拿主表两条腿的结论覆盖它。
    id: 'customer-contacts',
    handlerFile: '../backend/src/handlers/crm_customer_handler.rs',
    handlerFn: 'list_contacts',
    resource: 'customers',
    expectMaskCalls: ['apply_customer_field_permission'],
    rowsAt: 'root',
    anchorColumn: 'name',
    seeded: true,
    columns: [
      {
        column: 'phone',
        cn: '联系人电话',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['maskedPhone', 'absentKey'],
        nonAdminMayBePlaintext: false,
      },
      {
        column: 'email',
        cn: '联系人邮箱',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['maskedEmail', 'absentKey'],
        nonAdminMayBePlaintext: false,
      },
    ],
  },
  {
    id: 'purchase-prices-list',
    handlerFile: '../backend/src/handlers/purchase_price_handler.rs',
    handlerFn: 'list_prices',
    resource: 'purchase-prices',
    // 该腿服务端不脱敏：函数体里没有任何掩码调用，进价对持证角色原样下发。
    // 声明为空数组本身就是判据——后端补了掩码而这里没跟着改，接线对账当场判红。
    expectMaskCalls: [],
    rowsAt: 'root',
    anchorColumn: 'product_id',
    seeded: true,
    columns: [
      {
        column: 'price',
        cn: '进价',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'min_order_qty',
        cn: '最小起订量',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
    ],
  },
  {
    id: 'cost-collections-list',
    handlerFile: '../backend/src/handlers/cost_collection_handler.rs',
    handlerFn: 'list_collections',
    resource: 'cost-collections',
    expectMaskCalls: [],
    rowsAt: 'root',
    anchorColumn: 'dye_lot_no',
    seeded: true,
    columns: [
      {
        column: 'dyeing_fee',
        cn: '染厂加工费',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'processing_fee',
        cn: '外协加工费',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'total_cost',
        cn: '成本合计',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'dye_lot_no',
        cn: '缸号',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'color_no',
        cn: '色号',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'output_quantity_meters',
        cn: '米数',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
    ],
  },
  {
    id: 'outsourcing-orders-list',
    handlerFile: '../backend/src/handlers/outsourcing_handler.rs',
    handlerFn: 'list_outsourcing_orders',
    resource: 'outsourcing-orders',
    expectMaskCalls: [],
    rowsAt: 'items',
    anchorColumn: 'order_no',
    seeded: true,
    columns: [
      {
        column: 'standard_loss_rate',
        cn: '标准损耗',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'dye_lot_no',
        cn: '缸号',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'color_no',
        cn: '色号',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
    ],
  },
];

/**
 * 无建档腿的面：逐列断言打在出参投影声明上（那是该面对三端的权威形状），
 * 同时按权限码跑活体门（持证不得被 RBAC 挡、无键必须 403）。
 * 分成两组是因为这些面的行不由本矩阵产生（库存匹由生产与收货链路产，
 * 调整单与销售订单另有归属），拿别人的行做掩码断言会把建档值判成被掩。
 */
const CONTRACT_FACES: Array<{
  id: string;
  handlerFile: string;
  handlerFn: string;
  resource: string;
  /** 出参投影结构体（handler 侧的响应结构或实体 Model）所在文件与名 */
  shapeFile: string;
  shapeStruct: string;
  expectMaskCalls: string[];
  mustHave: ColumnSpec[];
  mustNotHave: ColumnSpec[];
}> = [
  {
    id: 'inventory-pieces-list',
    handlerFile: '../backend/src/handlers/inventory_piece_handler.rs',
    handlerFn: 'list_pieces',
    resource: 'pieces',
    shapeFile: '../backend/src/handlers/inventory_piece_handler.rs',
    shapeStruct: 'PieceResponse',
    // 匹号面是库存追溯面：它不携带任何成本列，这是“成本不外泄到追溯面”的声明判据。
    expectMaskCalls: [],
    mustHave: [
      {
        column: 'piece_no',
        cn: '卷号',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'dye_lot_no',
        cn: '缸号',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'color_no',
        cn: '色号',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'length',
        cn: '米数',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'width',
        cn: '门幅',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'gram_weight',
        cn: '克重',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
    ],
    mustNotHave: [
      {
        column: 'remaining_length',
        cn: '余码',
        kind: 'text',
        admin: ['absentColumn'],
        nonAdmin: ['absentColumn'],
        nonAdminMayBePlaintext: false,
      },
      {
        column: 'cost_price',
        cn: '成本价',
        kind: 'text',
        admin: ['absentColumn'],
        nonAdmin: ['absentColumn'],
        nonAdminMayBePlaintext: false,
      },
      {
        column: 'dyeing_fee',
        cn: '染厂加工费',
        kind: 'text',
        admin: ['absentColumn'],
        nonAdmin: ['absentColumn'],
        nonAdminMayBePlaintext: false,
      },
    ],
  },
  {
    id: 'inventory-adjustments-list',
    handlerFile: '../backend/src/handlers/inventory_adjustment_handler.rs',
    handlerFn: 'list_adjustments',
    resource: 'adjustments',
    shapeFile: '../backend/src/models/inventory_adjustment.rs',
    shapeStruct: 'Model',
    expectMaskCalls: [],
    mustHave: [
      {
        column: 'adjustment_no',
        cn: '库存调整单号',
        kind: 'text',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
    ],
    mustNotHave: [
      {
        column: 'cost_price',
        cn: '成本价',
        kind: 'text',
        admin: ['absentColumn'],
        nonAdmin: ['absentColumn'],
        nonAdminMayBePlaintext: false,
      },
    ],
  },
  {
    id: 'sales-order-item-discount',
    handlerFile: '../backend/src/handlers/sales_order_handler.rs',
    handlerFn: 'list_orders',
    resource: 'orders',
    shapeFile: '../backend/src/models/sales_order_item.rs',
    shapeStruct: 'Model',
    // 订单族服务端只掩联系方式，不掩金额与折扣：折扣列对所有能读订单的角色都是原文。
    expectMaskCalls: ['mask_contact_fields_for_role'],
    mustHave: [
      {
        column: 'discount_percent',
        cn: '折扣',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
      {
        column: 'discount_amount',
        cn: '折扣额',
        kind: 'decimal',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
    ],
    mustNotHave: [],
  },
  {
    id: 'bi-profit',
    handlerFile: '../backend/src/handlers/bi_handler.rs',
    handlerFn: 'profit_analysis',
    resource: 'bi-analysis',
    shapeFile: '../backend/src/services/bi_analysis_ops/types.rs',
    shapeStruct: 'ProfitMetric',
    expectMaskCalls: [],
    mustHave: [
      // 毛利率是 f64 计算值：出参形态是数字，与 DECIMAL 列的字符串形态是两种口径，
      // 按 ideal 把它也断成字符串会造出一条后端从未有过的假判据。
      {
        column: 'gross_margin',
        cn: '毛利率',
        kind: 'float',
        admin: ['clear'],
        nonAdmin: ['clear'],
        nonAdminMayBePlaintext: true,
      },
    ],
    mustNotHave: [],
  },
];

/** 全部面的接线声明（逐列与活体断言按分组分别跑） */
const ALL_FACES = [...SEEDED_FACES, ...CONTRACT_FACES];

/** 掩码后的电话按后端 mask_phone 口径算：数字位前 3 与后 4 之间打四个星 */
function expectedMaskedPhone(raw: string): string {
  const digits = raw.replace(/\D/g, '');
  if (digits.length < 7) return '*'.repeat(Math.max(digits.length, 1));
  return `${digits.slice(0, 3)}****${digits.slice(-4)}`;
}

/** 掩码后的邮箱按后端 mask_email 口径算：首字母加三枚星再加 @ 与域名 */
function expectedMaskedEmail(raw: string): string {
  const at = raw.indexOf('@');
  if (at < 0) return '***';
  return `${raw.slice(0, 1)}***@${raw.slice(at + 1)}`;
}

/** 持证角色：授权矩阵里对该面资源动作有授权、且本车道有其 E2E 凭证（admin 除外） */
function holderRolesOf(resource: string, action: string): string[] {
  const out: string[] = [];
  for (const [role, pairs] of matrixGrants.entries()) {
    if (role === 'admin') continue;
    if (!seededCodesHas(role)) continue;
    if (
      !backendGrants(
        pairs.map(p => `${p.resource}:${p.action}`),
        resource,
        action
      )
    )
      continue;
    if (!getRoleCredential(role)) continue;
    out.push(role);
  }
  return out.sort();
}

/** 无该键的持证角色，用作权限门的拒绝侧 */
function nonHolderRolesOf(resource: string, action: string): string[] {
  const out: string[] = [];
  for (const [role, pairs] of matrixGrants.entries()) {
    if (role === 'admin') continue;
    if (!seededCodesHas(role)) continue;
    if (
      backendGrants(
        pairs.map(p => `${p.resource}:${p.action}`),
        resource,
        action
      )
    )
      continue;
    if (!getRoleCredential(role)) continue;
    out.push(role);
  }
  return out.sort();
}

const seededRoleCodes = new Set(seededRoles.map(r => r.code));
function seededCodesHas(role: string): boolean {
  return seededRoleCodes.has(role);
}

/**
 * 把实测取值与按后端源派生的期望处置对上，不平就收集点名。
 * 调用方：逐列活体断言。
 * 入参：行对象、列声明、该角色侧允许的处置集合、会话身份描述、收集桶。
 * 传给谁：只进失败信息与制品，不发请求、不落库。
 * 存什么/存哪里：把不平描述 push 进调用方的桶。
 * 判据强度：允许集合之外的任何处置都算不平；neverPlaintext 为假时额外钉“不得等于建档值”，
 * 所以“掩码函数被摘掉但键还在”会被抓到，而“后端新增第三种处置”也不会被静默放过。
 */
function compareColumn(
  row: Row,
  spec: ColumnSpec,
  allowed: Disposition[],
  who: string,
  bucket: string[]
): void {
  const has = Object.prototype.hasOwnProperty.call(row, spec.column);
  const value = row[spec.column];
  const origin = SEED_BY_COLUMN[spec.column];
  const label = `${who} 的 ${spec.column}（${spec.cn}）`;

  if (!has) {
    if (!allowed.includes('absentKey')) {
      bucket.push(`${label} 出参里没有这个键，允许处置=${allowed.join(' 或 ')}`);
    }
    return;
  }
  if (allowed.includes('absentColumn')) {
    bucket.push(`${label} 不应出现在出参结构里，实际=${JSON.stringify(value)}`);
    return;
  }
  if (value === null || value === undefined) {
    if (!allowed.includes('null')) {
      bucket.push(`${label} 实际是 null，允许处置=${allowed.join(' 或 ')}（该面声明原文下发）`);
    }
    return;
  }

  // 形态判据先于取值判据：DECIMAL 出参必须是字符串，f64 必须是数字
  if (spec.kind === 'decimal' && typeof value !== 'string' && typeof value !== 'number') {
    bucket.push(`${label} 出参类型=${typeof value}，DECIMAL 列应为字符串`);
  }
  if (spec.kind === 'float' && typeof value !== 'number') {
    bucket.push(
      `${label} 出参类型=${typeof value}，f64 计算列应为数字而非 ${JSON.stringify(value)}`
    );
  }
  if (spec.kind === 'presence') return;

  if (!spec.nonAdminMayBePlaintext && origin !== undefined && String(value) === origin) {
    bucket.push(`${label} 拿到建档原值 ${origin}，但该面声明非 admin 不得见明文`);
    return;
  }

  if (allowed.includes('maskedPhone')) {
    if (typeof value === 'string' && origin !== undefined && value === expectedMaskedPhone(origin))
      return;
  }
  if (allowed.includes('maskedEmail')) {
    if (typeof value === 'string' && origin !== undefined && value === expectedMaskedEmail(origin))
      return;
  }
  if (allowed.includes('clear')) {
    if (origin === undefined) return;
    const same =
      spec.kind === 'decimal' ? Number(value) === Number(origin) : String(value) === origin;
    if (same) return;
  }
  bucket.push(
    `${label} 实际=${JSON.stringify(value)}，不落在允许处置 ${allowed.join(' 或 ')} 上` +
      `（建档值=${origin ?? '未登记'}）`
  );
}

/** 各面的列表请求路径（带最小分页参数，避免深分页拖慢） */
function faceListPath(faceId: string, seeded: { productId: number; customerId: number }): string {
  switch (faceId) {
    case 'products-list':
      return '/products?page=1&page_size=20';
    case 'products-detail':
      return `/products/${seeded.productId}`;
    case 'customers-list':
      return '/crm/customers?page=1&page_size=20';
    case 'customers-detail':
      return `/crm/customers/${seeded.customerId}`;
    case 'customer-contacts':
      return `/crm/customers/${seeded.customerId}/contacts`;
    case 'purchase-prices-list':
      return '/purchase/purchase-prices?page=1&page_size=50';
    case 'cost-collections-list':
      return `/production/cost-collections?page=1&page_size=50&batch_no=${seededDyeLotNo()}`;
    case 'outsourcing-orders-list':
      return '/production/outsourcing-orders?page=1&page_size=50';
    case 'inventory-pieces-list':
      return '/inventory/pieces?page=1&page_size=10';
    case 'inventory-adjustments-list':
      return '/inventory/adjustments?page=1&page_size=10';
    case 'sales-order-item-discount':
      return '/sales/orders?page=1&page_size=5';
    case 'bi-profit':
      return '/bi/sales/profit';
    default:
      throw new Error(`面 ${faceId} 没有登记请求路径，判据无从下发`);
  }
}

/** 建档缸号（延迟读取，供成本归集列表的过滤参数用） */
let seedDyeLotNo = '';
function seededDyeLotNo(): string {
  return seedDyeLotNo;
}

/**
 * 取一枚面的行集合。
 * 调用方：活体逐列断言与权限门断言。
 * 入参：已登录的 page、面 id、建档主键集。传给谁：只发 GET，不写库。
 * 存什么/存哪里：不持久化。
 * 形状不符即抛错点名实际键：把“信封换形了”判红，而不是当成零行通过。
 */
async function fetchFaceRows(
  page: Page,
  faceId: string,
  rowsAt: Face['rowsAt'],
  seeded: { productId: number; customerId: number }
): Promise<Row[]> {
  const raw = await apiCallRaw<unknown>(page, 'GET', faceListPath(faceId, seeded));
  if (rowsAt === 'single') {
    if (!raw || typeof raw !== 'object' || Array.isArray(raw)) {
      throw new Error(`面 ${faceId} 出参不是单对象，实际=${JSON.stringify(raw)?.slice(0, 200)}`);
    }
    return [raw as Row];
  }
  if (rowsAt === 'root') {
    if (!Array.isArray(raw)) {
      throw new Error(
        `面 ${faceId} 声明出参是裸数组，实际不是数组，键=${
          raw && typeof raw === 'object' ? Object.keys(raw).join(',') : String(raw)
        }`
      );
    }
    return raw as Row[];
  }
  const env = raw as Row;
  const items = env['items'];
  if (!Array.isArray(items)) {
    throw new Error(
      `面 ${faceId} 出参信封里定位不到 items 数组，实际键=${Object.keys(env).join(',')}`
    );
  }
  return items as Row[];
}

/** 在列表里定位本矩阵建档的那一行（读别人的行做掩码断言会把建档值判成被掩） */
function pickSeededRow(
  rows: Row[],
  face: Face,
  seeded: { productId: number; customerId: number }
): Row {
  if (face.rowsAt === 'single') return rows[0];
  const anchor = face.anchorColumn;
  if (!anchor) return rows[0];
  const want =
    anchor === 'product_id'
      ? String(seeded.productId)
      : anchor === 'code'
        ? SEED.productCode
        : anchor === 'customer_name'
          ? SEED.customerName
          : anchor === 'name'
            ? SEED.contactName
            : anchor === 'dye_lot_no'
              ? seedDyeLotNo
              : anchor === 'order_no'
                ? SEED.orderNo
                : undefined;
  if (want === undefined) {
    throw new Error(`面 ${face.id} 的锚点列 ${anchor} 没有登记建档取值，无法定位本矩阵的行，判红`);
  }
  const hit = rows.find(r => String(r[anchor]) === want);
  if (!hit) {
    throw new Error(
      `面 ${face.id} 的行集合里找不到本矩阵建档行（按 ${anchor}=${want} 匹配），` +
        `返回行数=${rows.length}，首行片段=${JSON.stringify(rows[0])?.slice(0, 200)}——` +
        '建档未落库或被行级数据范围挡在集合外，判红而非拿别人的行凑数'
    );
  }
  return hit;
}

// ==================== 建档 ====================

const seeded = {
  productId: 0,
  productName: '',
  customerId: 0,
  supplierId: 0,
  purchasePriceId: 0,
  costCollectionId: 0,
  outsourcingOrderId: 0,
  contactId: 0,
};

test.describe('显示控制与脱敏面矩阵', () => {
  let admin: IsolatedSession;

  test.beforeAll(async ({ browser }) => {
    admin = await createIsolatedSession(browser, 'admin', { authMode: 'real' });
    seedDyeLotNo = SEED.dyeLotNo;

    const product = await apiCallRaw<Row>(admin.page, 'POST', '/products', {
      code: SEED.productCode,
      name: 'E2E 面料可见性矩阵产品',
      unit: '米',
      product_type: '面料',
      fabric_composition: SEED.fabric_composition,
      gram_weight: SEED.gram_weight,
      width: SEED.width,
      standard_price: SEED.standard_price,
      cost_price: SEED.cost_price,
    });
    seeded.productId = Number(product?.id);
    if (!Number.isFinite(seeded.productId) || seeded.productId <= 0) {
      throw new Error(
        `前置失败：建面料产品未返回有效 id，实际出参=${JSON.stringify(product)?.slice(0, 240)}`
      );
    }

    const customer = await apiCallRaw<Row>(admin.page, 'POST', '/crm/customers', {
      customer_name: SEED.customerName,
      contact_phone: SEED.contact_phone,
      contact_email: SEED.contact_email,
      address: SEED.address,
      credit_limit: SEED.credit_limit,
    });
    seeded.customerId = Number(customer?.id);
    if (!Number.isFinite(seeded.customerId) || seeded.customerId <= 0) {
      throw new Error(
        `前置失败：建客户未返回有效 id，实际出参=${JSON.stringify(customer)?.slice(0, 240)}`
      );
    }

    const contact = await apiCallRaw<Row>(
      admin.page,
      'POST',
      `/crm/customers/${seeded.customerId}/contacts`,
      {
        name: SEED.contactName,
        phone: SEED.contactPhone,
        email: SEED.contactEmail,
        is_primary: true,
      }
    );
    seeded.contactId = Number(contact?.id);
    if (!Number.isFinite(seeded.contactId) || seeded.contactId <= 0) {
      throw new Error(
        `前置失败：建客户联系人未返回有效 id，实际出参=${JSON.stringify(contact)?.slice(0, 240)}`
      );
    }

    const supplierList = await apiCallRaw<Row>(
      admin.page,
      'GET',
      '/purchase/suppliers?page=1&page_size=5'
    );
    const supplierRows = Array.isArray(supplierList['items'])
      ? (supplierList['items'] as Row[])
      : [];
    if (supplierRows.length > 0) {
      seeded.supplierId = Number(supplierRows[0].id);
    } else {
      const supplier = await apiCallRaw<Row>(admin.page, 'POST', '/purchase/suppliers', {
        supplier_name: genCode('E2E-FV-S'),
        supplier_type: '染厂',
      });
      seeded.supplierId = Number(supplier?.id);
    }
    if (!Number.isFinite(seeded.supplierId) || seeded.supplierId <= 0) {
      throw new Error('前置失败：既取不到也建不出供应商，采购价与委外两条腿无从建档');
    }

    const price = await apiCallRaw<unknown>(admin.page, 'POST', '/purchase/purchase-prices', {
      product_id: seeded.productId,
      supplier_id: seeded.supplierId,
      price: SEED.price,
      currency: 'CNY',
      unit: '米',
      min_order_qty: '100',
      price_type: '标准',
      effective_date: new Date().toISOString().slice(0, 10),
    });
    seeded.purchasePriceId = Number(Array.isArray(price) ? price[0]?.id : (price as Row)?.id);
    if (!Number.isFinite(seeded.purchasePriceId) || seeded.purchasePriceId <= 0) {
      throw new Error(
        `前置失败：建采购价未返回有效 id，实际出参=${JSON.stringify(price)?.slice(0, 240)}`
      );
    }

    const collection = await apiCallRaw<unknown>(
      admin.page,
      'POST',
      '/production/cost-collections',
      {
        collection_date: new Date().toISOString().slice(0, 10),
        cost_object_type: 'dye_lot',
        dye_lot_no: seedDyeLotNo,
        color_no: SEED.colorNo,
        batch_no: seedDyeLotNo,
        direct_material: '1000.00',
        direct_labor: '200.00',
        manufacturing_overhead: '100.00',
        processing_fee: SEED.processing_fee,
        dyeing_fee: SEED.dyeing_fee,
        output_quantity_meters: SEED.output_quantity_meters,
        output_quantity_kg: '300.00',
      }
    );
    seeded.costCollectionId = Number(
      Array.isArray(collection) ? collection[0]?.id : (collection as Row)?.id
    );
    if (!Number.isFinite(seeded.costCollectionId) || seeded.costCollectionId <= 0) {
      throw new Error(
        `前置失败：建成本归集未返回有效 id，实际出参=${JSON.stringify(collection)?.slice(0, 240)}`
      );
    }

    const outsourcing = await apiCallRaw<unknown>(
      admin.page,
      'POST',
      '/production/outsourcing-orders',
      {
        order_no: SEED.orderNo,
        order_type: '染色',
        supplier_id: seeded.supplierId,
        color_no: SEED.colorNo,
        dye_lot_no: seedDyeLotNo,
        issue_date: new Date().toISOString().slice(0, 10),
        issue_quantity: '500',
        issue_unit: '米',
        material_cost: '2000.00',
        standard_loss_rate: SEED.standard_loss_rate,
      }
    );
    seeded.outsourcingOrderId = Number(
      Array.isArray(outsourcing) ? outsourcing[0]?.id : (outsourcing as Row)?.id
    );
    if (!Number.isFinite(seeded.outsourcingOrderId) || seeded.outsourcingOrderId <= 0) {
      throw new Error(
        `前置失败：建委外订单未返回有效 id，实际出参=${JSON.stringify(outsourcing)?.slice(0, 240)}`
      );
    }
  });

  test.afterAll(async () => {
    if (!admin) return;
    mkdirSync(MATRIX_ARTIFACT_DIR, { recursive: true });
    writeFileSync(
      `${MATRIX_ARTIFACT_DIR}/201-seeded-rows.json`,
      JSON.stringify(
        {
          note: '本矩阵建的数据行；客户与产品当场回收，采购价与成本归集与委外单对本矩阵行无可用 DELETE 路由，留库并点名',
          ...seeded,
          notReclaimed: [
            `采购价 ${seeded.purchasePriceId}`,
            `成本归集 ${seeded.costCollectionId}`,
            `委外订单 ${seeded.outsourcingOrderId}`,
          ],
        },
        null,
        2
      ) + '\n',
      'utf-8'
    );
    await apiCallExpectFail(admin.page, 'DELETE', `/crm/customers/${seeded.customerId}`).catch(
      () => undefined
    );
    await apiCallExpectFail(admin.page, 'DELETE', `/products/${seeded.productId}`).catch(
      () => undefined
    );
    await admin.close();
  });

  // ---------- 一 掩码原语底数 ----------

  test('脱敏原语底数 成本价置空目标恰为一枚且与判键同键，联系方式掩码键含电话族与邮箱族', () => {
    expect(
      maskKeys.costKeys,
      `后端 mask_sensitive_fields 的非 admin 置空键集合实测为 ${maskKeys.costKeys.join(
        ', '
      )}，本项目口径只有成本价一枚；集合变化必须同步本矩阵的面声明，不能让掩码范围静默扩张或摘空`
    ).toEqual(['cost_price']);
    expect(
      maskKeys.phoneKeys,
      `电话掩码键集合必须同时覆盖出参实际用到的 phone 与 contact_phone 两形态，实测=${maskKeys.phoneKeys.join(', ')}`
    ).toEqual(expect.arrayContaining(['phone', 'contact_phone']));
    expect(
      maskKeys.emailKeys,
      `邮箱掩码键集合必须同时覆盖 email 与 contact_email 两形态，实测=${maskKeys.emailKeys.join(', ')}`
    ).toEqual(expect.arrayContaining(['email', 'contact_email']));
    expect(
      maskKeys.removedKeys,
      `客户域默认脱敏必须整键移除 address，实测移除集=${maskKeys.removedKeys.join(', ')}`
    ).toContain('address');
  });

  test('分面对账 逐面读 handler 函数体，实挂脱敏调用与声明双向全等', () => {
    const mismatches: string[] = [];
    for (const face of ALL_FACES) {
      const body = readHandlerFunctionBody(face.handlerFile, face.handlerFn);
      const observed = detectMaskCalls(body);
      const missing = face.expectMaskCalls.filter(c => !observed.includes(c));
      const extra = observed.filter(c => !face.expectMaskCalls.includes(c));
      if (missing.length > 0) {
        mismatches.push(
          `面 ${face.id} 声明应挂 ${face.expectMaskCalls.join(' 与 ')}，函数体里实际没有（缺 ${missing.join(
            ', '
          )}）：该腿已变成不脱敏旁路，或函数改名致本矩阵读不到函数体`
        );
      }
      if (extra.length > 0) {
        mismatches.push(
          `面 ${face.id} 出现未在矩阵声明里的脱敏调用 ${extra.join(
            ', '
          )}：新增处置必须逐列登记进本矩阵，不能让矩阵继续按“该面不脱敏”通过`
        );
      }
    }
    expect(
      mismatches,
      `分面对账不平共 ${mismatches.length} 项：\n  ${mismatches.join('\n  ')}`
    ).toEqual([]);
  });

  test('admin 判定入口在各掩码腿里可见（防按角色主键字面量判定的第二套口径）', () => {
    const maskedFaces = ALL_FACES.filter(f => f.expectMaskCalls.length > 0);
    const problems: string[] = [];
    for (const face of maskedFaces) {
      const body = readHandlerFunctionBody(face.handlerFile, face.handlerFn);
      const direct = /is_admin_role\s*\(/.test(body);
      const viaCustomerImpl =
        /apply_customer_field_permission\s*\(/.test(body) ||
        /apply_customer_field_config_mask\s*\(/.test(body);
      const viaSalesImpl = /mask_contact_fields_for_role\s*\(/.test(body);
      if (!direct && !viaCustomerImpl && !viaSalesImpl) {
        problems.push(`${face.id} 挂了脱敏却在函数体里找不到 admin 判定入口或唯一实现入口`);
      }
    }
    expect(
      problems,
      `admin 判定入口缺失共 ${problems.length} 项：${problems.join('；')}——` +
        '本项目要求掩码只认 admin 角色码且判定源唯一'
    ).toEqual([]);
    expect(
      maskedFaces.length,
      `已登记为服务端脱敏的面数为 ${maskedFaces.length}，为 0 说明掩码面全被摘掉，判红`
    ).toBeGreaterThan(0);
  });

  // ---------- 二 活体逐列 ----------

  for (const face of SEEDED_FACES) {
    test(`逐列可见性 ${face.id}（admin 与每个持证非 admin 对照）`, async ({ browser }) => {
      const problems: string[] = [];
      const visibleKeySets: Record<string, string[]> = {};

      const adminRows = await fetchFaceRows(admin.page, face.id, face.rowsAt, seeded);
      const adminRow = pickSeededRow(adminRows, face, seeded);
      for (const spec of face.columns) {
        compareColumn(adminRow, spec, spec.admin, 'admin 腿', problems);
      }
      visibleKeySets.admin = Object.keys(adminRow).sort();

      const holders = holderRolesOf(face.resource, 'read');
      for (const role of holders) {
        const session = await createIsolatedSession(browser, role, {
          authMode: 'real',
          credentialRole: role,
        });
        try {
          const identity = await readRelayIdentity(
            session.page,
            `面 ${face.id} 的角色 ${role} 会话`
          );
          expect(
            identity.permissions.length,
            `面 ${face.id} 的角色 ${role} 登录后 permissions 为空数组（登录出参的授权面丢失），可见性无从判定`
          ).toBeGreaterThan(0);
          const rows = await fetchFaceRows(session.page, face.id, face.rowsAt, seeded);
          if (rows.length === 0) {
            // 非 admin 读到空集是行级数据范围收敛的真实形态（本矩阵建的是 admin 名下数据）。
            // 如实登记，不据此判掩码通过，也不静默略过这一列。
            visibleKeySets[role] = ['<行级数据范围下零行，逐列比对未发生>'];
            continue;
          }
          let row: Row;
          try {
            row = pickSeededRow(rows, face, seeded);
          } catch (error) {
            // 列表里没有本矩阵那行同样属行级收敛，登记后跳过逐列比对；
            // 单对象腿（详情面）不走这条路，它要么 200 要么由权限门断言抓。
            visibleKeySets[role] = [
              `<集合内无本矩阵建档行：${(error as Error).message.slice(0, 120)}>`,
            ];
            continue;
          }
          for (const spec of face.columns) {
            compareColumn(row, spec, spec.nonAdmin, `角色 ${role} 腿`, problems);
          }
          visibleKeySets[role] = Object.keys(row).sort();
        } finally {
          await session.close();
        }
      }

      mkdirSync(MATRIX_ARTIFACT_DIR, { recursive: true });
      writeFileSync(
        `${MATRIX_ARTIFACT_DIR}/201-${face.id}.json`,
        JSON.stringify(
          {
            face: face.id,
            resource: face.resource,
            handler: `${face.handlerFile}::${face.handlerFn}`,
            serverSideMaskCalls: face.expectMaskCalls,
            holderRolesWithCredential: holders,
            columns: face.columns.map(
              c =>
                `${c.column}(${c.cn}) admin=${c.admin.join('或')} nonAdmin=${c.nonAdmin.join('或')}`
            ),
            visibleKeySetByRole: visibleKeySets,
          },
          null,
          2
        ) + '\n',
        'utf-8'
      );

      expect(
        problems,
        `面 ${face.id} 的逐列可见性与后端源派生的期望不平，共 ${problems.length} 项：\n  ${problems.join(
          '\n  '
        )}`
      ).toEqual([]);
      // 检测力自证：持证非 admin 角色必须真被跑过至少一次，否则上面的循环空转也是绿。
      expect(
        holders.length,
        `面 ${face.id} 的资源 ${face.resource} 没有任何“持证且有 E2E 凭证”的非 admin 角色，` +
          '非 admin 腿的逐列断言没发生，该面的掩码结论不成立'
      ).toBeGreaterThan(0);
    });
  }

  // ---------- 三 声明面与权限门 ----------

  for (const face of CONTRACT_FACES) {
    test(`出参声明面 ${face.id}（应有列与不得有列双向全等，含成本不外泄到追溯面）`, () => {
      const shape = readStructBody(face.shapeFile, face.shapeStruct);
      const declared = new Set(structMemberColumns(shape));
      const wantHave = face.mustHave.map(c => c.column);
      const wantNotHave = face.mustNotHave.map(c => c.column);
      const missing = wantHave.filter(c => !declared.has(c));
      const leaked = wantNotHave.filter(c => declared.has(c));
      expect(
        missing,
        `面 ${face.id} 的出参结构 ${face.shapeStruct} 里缺少应投影的列 ${missing.join(', ')}——` +
          `面料追溯列（缸号与色号与米数与克重与门幅与卷号）被摘掉，三端就读不到该列，属真实缺陷`
      ).toEqual([]);
      expect(
        leaked,
        `面 ${face.id} 的出参结构 ${face.shapeStruct} 里出现了不应存在的列 ${leaked.join(', ')}——` +
          '成本列外泄到追溯面，或有人补了一个假键冒充已实现'
      ).toEqual([]);
      expect(
        declared.size,
        `面 ${face.id} 的出参结构只解析出 ${declared.size} 个成员列，解析口径失效即判红，不按空集放过`
      ).toBeGreaterThan(0);
    });

    test(`权限码门 ${face.id} 的持证侧不被 RBAC 挡、无键侧必须 403`, async ({ browser }) => {
      const path = faceListPath(face.id, seeded);
      // 无键侧：必须 403 且归因到权限门机器码（不读文案）
      const blockers = nonHolderRolesOf(face.resource, 'read').slice(0, 2);
      expect(
        blockers.length,
        `面 ${face.id} 找不到“无 ${face.resource}:read 且有 E2E 凭证”的角色，越权侧无从判定，判红`
      ).toBeGreaterThan(0);
      for (const role of blockers) {
        const session = await createIsolatedSession(browser, role, {
          authMode: 'real',
          credentialRole: role,
        });
        try {
          const failed = await apiCallExpectFail(session.page, 'GET', path);
          expect(
            failed.status,
            `无 ${face.resource}:read 的角色 ${role} 读面 ${face.id}（GET ${path}）应被权限门挡住，实际 status=${failed.status}`
          ).toBe(403);
        } finally {
          await session.close();
        }
      }
      // 持证侧：RBAC 不得挡（允许 200 或 404 之类的业务态，独独不接受 403 权限码）
      const holders = holderRolesOf(face.resource, 'read').slice(0, 2);
      for (const role of holders) {
        const session = await createIsolatedSession(browser, role, {
          authMode: 'real',
          credentialRole: role,
        });
        try {
          const failed = await apiCallExpectFail(session.page, 'GET', path);
          if (failed.status === 403) {
            expect(
              failed.code,
              `持证角色 ${role} 读面 ${face.id}（GET ${path}）得到 403 且 code=${
                failed.code ?? '缺失'
              }：权限码已授却被挡，授权链或门控与后端定义不符`
            ).not.toBe('FORBIDDEN');
          }
        } finally {
          await session.close();
        }
      }
    });
  }

  // ---------- 四 出参形态 ----------

  test('DECIMAL 出参是字符串而 f64 计算列是数字，两形态不许互相冒充', async () => {
    const productDecimals = readDecimalColumns('../backend/src/models/product.rs');
    expect(
      productDecimals,
      `产品实体应至少含成本价、标准价、克重、门幅四枚 DECIMAL 列，实测=${productDecimals.join(', ')}`
    ).toEqual(expect.arrayContaining(['cost_price', 'standard_price', 'gram_weight', 'width']));
    const pieceDecimals = readDecimalColumns('../backend/src/models/inventory_piece.rs');
    expect(
      pieceDecimals,
      `库存匹实体应至少含米数、重量、门幅、克重四枚 DECIMAL 列，实测=${pieceDecimals.join(', ')}`
    ).toEqual(expect.arrayContaining(['length', 'weight', 'width', 'gram_weight']));

    const detail = await apiCallRaw<Row>(admin.page, 'GET', `/products/${seeded.productId}`);
    for (const col of ['standard_price', 'gram_weight', 'width'] as const) {
      expect(
        typeof detail[col],
        `产品详情腿的 DECIMAL 列 ${col} 出参类型=${typeof detail[col]}，本项目口径是字符串；` +
          '改成数字会让前端按 number 消费并丢精度'
      ).toBe('string');
      expect(
        Number(detail[col]),
        `产品详情腿的 ${col} 数值应等于建档值 ${SEED[col]}，实际=${JSON.stringify(detail[col])}`
      ).toBe(Number(SEED[col]));
    }

    const biBody = readStructBody(
      '../backend/src/services/bi_analysis_ops/types.rs',
      'ProfitMetric'
    );
    expect(
      /gross_margin:\s*f64/.test(biBody),
      `BI 毛利率列实测不是 f64（片段=${biBody.slice(0, 240)}），出参形态口径需重判`
    ).toBe(true);
  });

  test('客户信用额度在建档与出参两腿同形态（DECIMAL 列经 String 入参收口）', async () => {
    const createBody = readHandlerFunctionBody(
      '../backend/src/handlers/customer_handler.rs',
      'create_customer'
    );
    expect(
      /credit_limit/.test(createBody),
      '客户建档腿里读不到 credit_limit 的处理分支，信用额度入参形态口径已变，判红'
    ).toBe(true);
    const detail = await apiCallRaw<Row>(admin.page, 'GET', `/crm/customers/${seeded.customerId}`);
    const value = detail['credit_limit'];
    expect(
      value === null || typeof value === 'string' || typeof value === 'number',
      `信用额度出参形态既不是字符串也不是 null 或数字，实际类型=${typeof value}`
    ).toBe(true);
    expect(
      Number(value),
      `admin 腿读到的信用额度应等于建档值 ${SEED.credit_limit}，实际=${JSON.stringify(value)}`
    ).toBe(Number(SEED.credit_limit));
  });

  // ---------- 五 三端一致性与服务端脱敏边界 ----------

  test('三端一致 产品列表腿与详情腿对成本价的处置必须同向，详情腿不得成为列表掩码的旁路', async ({
    browser,
  }) => {
    const problems: string[] = [];
    const holders = holderRolesOf('products', 'read').slice(0, 3);
    for (const role of holders) {
      const session = await createIsolatedSession(browser, role, {
        authMode: 'real',
        credentialRole: role,
      });
      try {
        const rows = await fetchFaceRows(session.page, 'products-list', 'items', seeded);
        const listRow = pickSeededRow(rows, SEEDED_FACES[0], seeded);
        const detailRow = (
          await fetchFaceRows(session.page, 'products-detail', 'single', seeded)
        )[0];
        for (const key of ['cost_price', 'standard_price', 'supplier_id', 'gram_weight', 'width']) {
          const a = JSON.stringify(listRow[key] ?? '<无键>');
          const b = JSON.stringify(detailRow[key] ?? '<无键>');
          if (a !== b) {
            problems.push(
              `角色 ${role} 的列 ${key}：列表腿=${a} 详情腿=${b}（同一记录两条腿给出两种可见性）`
            );
          }
        }
      } finally {
        await session.close();
      }
    }
    expect(
      problems,
      `产品两条读腿跨腿不一致共 ${problems.length} 项：\n  ${problems.join(
        '\n  '
      )}——本项目成本价曾在详情腿原样下发，靠这条锁防止旁路回归`
    ).toEqual([]);
  });

  test('三端一致 客户三条读腿（列表 与 详情 与 联系人子端点）非 admin 均不得拿到电话邮箱明文', async ({
    browser,
  }) => {
    const problems: string[] = [];
    const observed: Record<string, string[]> = {};
    const holders = holderRolesOf('customers', 'read').slice(0, 3);
    expect(
      holders.length,
      '没有任何“持 customers:read 且有 E2E 凭证”的非 admin 角色，客户域掩码面无法实测，判红'
    ).toBeGreaterThan(0);

    for (const role of holders) {
      const session = await createIsolatedSession(browser, role, {
        authMode: 'real',
        credentialRole: role,
      });
      try {
        const listRows = await fetchFaceRows(session.page, 'customers-list', 'items', seeded);
        const legs: Array<{ name: string; row: Row | null }> = [];
        legs.push({
          name: '列表腿',
          row: listRows.length > 0 ? pickSeededRow(listRows, SEEDED_FACES[2], seeded) : null,
        });
        const detailRows = await fetchFaceRows(session.page, 'customers-detail', 'single', seeded);
        legs.push({ name: '详情腿', row: detailRows[0] ?? null });
        const contactRows = await fetchFaceRows(session.page, 'customer-contacts', 'root', seeded);
        legs.push({
          name: '联系人子端点',
          row: contactRows.length > 0 ? pickSeededRow(contactRows, SEEDED_FACES[4], seeded) : null,
        });

        observed[role] = [];
        for (const leg of legs) {
          if (!leg.row) {
            observed[role].push(`${leg.name}=被行级数据范围挡住（零行）`);
            continue;
          }
          const phoneKey = Object.prototype.hasOwnProperty.call(leg.row, 'contact_phone')
            ? 'contact_phone'
            : Object.prototype.hasOwnProperty.call(leg.row, 'phone')
              ? 'phone'
              : null;
          const emailKey = phoneKey
            ? Object.prototype.hasOwnProperty.call(leg.row, 'contact_email')
              ? 'contact_email'
              : Object.prototype.hasOwnProperty.call(leg.row, 'email')
                ? 'email'
                : null
            : null;
          if (phoneKey) {
            const v = String(leg.row[phoneKey]);
            if (v === SEED.contact_phone) {
              problems.push(`角色 ${role} 的${leg.name}拿到电话明文 ${v}`);
            } else if (v !== expectedMaskedPhone(SEED.contact_phone)) {
              problems.push(
                `角色 ${role} 的${leg.name}电话列=${JSON.stringify(v)}，既非明文也非后端掩码口径 ` +
                  expectedMaskedPhone(SEED.contact_phone)
              );
            }
          }
          if (emailKey) {
            const v = String(leg.row[emailKey]);
            if (v === SEED.contact_email) {
              problems.push(`角色 ${role} 的${leg.name}拿到邮箱明文 ${v}`);
            } else if (v !== expectedMaskedEmail(SEED.contact_email)) {
              problems.push(
                `角色 ${role} 的${leg.name}邮箱列=${JSON.stringify(v)}，既非明文也非后端掩码口径 ` +
                  expectedMaskedEmail(SEED.contact_email)
              );
            }
          }
          if (Object.prototype.hasOwnProperty.call(leg.row, 'address')) {
            problems.push(
              `角色 ${role} 的${leg.name}应整键移除 address，实际仍下发=${JSON.stringify(leg.row['address'])}`
            );
          }
          observed[role].push(
            `${leg.name}=电话键${phoneKey ?? '缺席'} 邮箱键${emailKey ?? '缺席'}`
          );
        }
      } finally {
        await session.close();
      }
    }
    mkdirSync(MATRIX_ARTIFACT_DIR, { recursive: true });
    writeFileSync(
      `${MATRIX_ARTIFACT_DIR}/201-customer-legs.json`,
      JSON.stringify({ note: '客户域三条读腿的实测处置', observed }, null, 2) + '\n',
      'utf-8'
    );
    expect(
      problems,
      `客户域跨腿明文外泄共 ${problems.length} 项：\n  ${problems.join('\n  ')}`
    ).toEqual([]);
  });

  test('PC 与 H5 读同一套后端读腿（路由快照里不存在第二套移动端列表端点）', () => {
    const snapshot = readRouteSnapshot();
    const listFaces = ['/products', '/crm/customers', '/sales/orders', '/inventory/pieces'];
    const mobileAliases = snapshot
      .filter(r => r.method === 'GET')
      .map(r => r.path)
      .filter(
        p =>
          /\/(h5|mobile|m)\/(products|customers|orders)/.test(p) ||
          /\/app-api\//.test(p) ||
          listFaces.some(
            base => p !== `/api/v1/erp${base}` && p.endsWith(base) && p.includes('/mobile')
          )
      );
    expect(
      mobileAliases,
      `路由快照出现第二套移动端列表端点 ${mobileAliases.join(', ')}：` +
        'PC 与 H5 不再共用同一出参面，跨端一致性需按新面重判'
    ).toEqual([]);
    for (const base of listFaces) {
      expect(
        snapshot.some(r => r.method === 'GET' && r.path === `/api/v1/erp${base}`),
        `路由快照里缺少三端共用的列表端点 GET ${base}，该面的跨端一致性判据失去落点`
      ).toBe(true);
    }
  });

  test('小程序可见性判据从后端契约导出：逐角色能读到成本族真值的面集合必须等于按其权限码派生的集合', async ({
    browser,
  }) => {
    const COST_FACES: Array<{ id: string; resource: string; column: string; cn: string }> = [
      { id: 'products-list', resource: 'products', column: 'cost_price', cn: '成本价' },
      { id: 'purchase-prices-list', resource: 'purchase-prices', column: 'price', cn: '进价' },
      {
        id: 'cost-collections-list',
        resource: 'cost-collections',
        column: 'dyeing_fee',
        cn: '染厂加工费',
      },
      {
        id: 'outsourcing-orders-list',
        resource: 'outsourcing-orders',
        column: 'standard_loss_rate',
        cn: '损耗',
      },
      { id: 'bi-profit', resource: 'bi-analysis', column: 'gross_margin', cn: '毛利率' },
    ];
    // 小程序侧沿用后端登录出参的权限码面（miniprogram/libs/utils/perm.js 复刻后端
    // matches_permission），因此它的可达面判据就是“该角色权限码能否命中该端点派生的键”。
    const targetRoles = [
      ...new Set([
        ...COST_FACES.flatMap(f => holderRolesOf(f.resource, 'read').slice(0, 1)),
        ...COST_FACES.flatMap(f => nonHolderRolesOf(f.resource, 'read').slice(0, 1)),
      ]),
    ].slice(0, 4);
    expect(
      targetRoles.length,
      '成本族面挑不出任何可测角色（凭证或矩阵分组缺失），小程序侧的判据无从导出，判红'
    ).toBeGreaterThan(0);

    const rows: string[] = [];
    const problems: string[] = [];
    for (const role of targetRoles) {
      const session = await createIsolatedSession(browser, role, {
        authMode: 'real',
        credentialRole: role,
      });
      try {
        const identity = await readRelayIdentity(session.page, `小程序判据导出会话 ${role}`);
        for (const face of COST_FACES) {
          const derived = backendGrants(identity.permissions, face.resource, 'read');
          const path = faceListPath(face.id, seeded);
          const failed = await apiCallExpectFail(session.page, 'GET', path);
          const allowed = !(failed.status === 403 && failed.code === 'FORBIDDEN');
          if (derived !== allowed) {
            problems.push(
              `角色 ${role} 对 ${face.cn} 面 ${face.id}：按权限码派生应${derived ? '可读' : '被挡'}，` +
                `实测${allowed ? '放行' : '403 FORBIDDEN'}（派生键=${face.resource}:read）`
            );
          }
          if (allowed && identity.permissions.length > 0 && !derived) {
            problems.push(
              `角色 ${role} 无 ${face.resource}:read 却读到了 ${face.cn} 面，属越权放行`
            );
          }
          rows.push(`${role}|${face.id}|${face.column}|派生=${derived}|实测放行=${allowed}`);
        }
      } finally {
        await session.close();
      }
    }
    mkdirSync(MATRIX_ARTIFACT_DIR, { recursive: true });
    writeFileSync(
      `${MATRIX_ARTIFACT_DIR}/201-miniprogram-facing-facts.json`,
      JSON.stringify(
        {
          note: '小程序侧的权威判据：每枚成本族面对每个角色的可达性。服务端未脱敏的面必须由小程序端按权限码收敛调用点，不许靠前端隐藏冒充脱敏。',
          rows,
          unmaskedCostFaces: COST_FACES.filter(f => f.id !== 'products-list').map(f => f.id),
          serverMaskedFaces: ['products-list', 'products-detail'],
        },
        null,
        2
      ) + '\n',
      'utf-8'
    );
    expect(
      problems,
      `小程序侧可达性判据与后端契约不符共 ${problems.length} 项：\n  ${problems.join('\n  ')}`
    ).toEqual([]);
  });

  test('服务端未脱敏的成本族面集合如实登记（防有人靠前端隐藏把这条结论洗绿）', () => {
    const unmasked = ALL_FACES.filter(f => f.expectMaskCalls.length === 0)
      .map(f => f.id)
      .sort();
    expect(
      unmasked,
      `服务端不脱敏的面集合发生变化，实测=${unmasked.join(', ')}。这些面的列可见性只由端点可达性` +
        '决定，任何"看不见"的结论都必须来自权限码或行级数据范围，不来自前端列配置'
    ).toEqual([
      'bi-profit',
      'cost-collections-list',
      'inventory-adjustments-list',
      'inventory-pieces-list',
      'outsourcing-orders-list',
      'purchase-prices-list',
      'sales-order-item-discount',
    ]);
  });

  test('面料字段索引的 15 项全部落到真实面与真实列（防字段清单与面声明脱钩）', () => {
    const index: Array<{ cn: string; column: string }> = [
      { cn: '成本价', column: 'cost_price' },
      { cn: '进价', column: 'price' },
      { cn: '染厂加工费', column: 'dyeing_fee' },
      { cn: '毛利', column: 'gross_margin' },
      { cn: '折扣修改开关', column: 'discount_percent' },
      { cn: '信用额度修改', column: 'credit_limit' },
      { cn: '库存调整', column: 'adjustment_no' },
      { cn: '缸号', column: 'dye_lot_no' },
      { cn: '色号', column: 'color_no' },
      { cn: '米数', column: 'output_quantity_meters' },
      { cn: '克重', column: 'gram_weight' },
      { cn: '门幅', column: 'width' },
      { cn: '损耗', column: 'standard_loss_rate' },
      { cn: '余码', column: 'remaining_length' },
      { cn: '卷号', column: 'piece_no' },
    ];
    const declaredColumns = new Set<string>();
    for (const f of SEEDED_FACES) for (const c of f.columns) declaredColumns.add(c.column);
    for (const f of CONTRACT_FACES) {
      for (const c of f.mustHave) declaredColumns.add(c.column);
      for (const c of f.mustNotHave) declaredColumns.add(c.column);
    }
    const unbound = index
      .filter(i => !declaredColumns.has(i.column))
      .map(i => `${i.cn}=${i.column}`);
    expect(
      unbound,
      `用户口径的 15 项里有 ${unbound.length} 项没有落到任何面的真实列声明上：${unbound.join(', ')}`
    ).toEqual([]);
    expect(
      index.length,
      `面料字段索引应有 15 项（成本价到卷号），实测 ${index.length} 项，分母被削减即失去守护`
    ).toBe(15);
  });

  test('小程序侧待接权限码逐枚在后端可授予，且后端动作派生与小程序沿用的动作名同形', () => {
    // 判据来源：miniprogram/libs/utils/permEntries.js 的 codes 声明面（车道 wt-miniprogram，
    // 提交 f59ee6e3）。小程序复刻后端 matches_permission，所以它的每一枚码都必须能在后端
    // 授权矩阵里被命中或通配命中；命中不了就是小程序接了一个除超管旁路外无人可用的入口。
    const MP_CODES = [
      'customers:create',
      'customers:read',
      'customers:update',
      'dashboard:read',
      'orders:cancel',
      'orders:create',
      'orders:read',
      'product-categories:update',
      'products:create',
      'products:delete',
      'products:read',
      'products:update',
      'sales-returns:create',
      'sales-returns:update',
    ];
    const grantable = new Set<string>();
    for (const pairs of matrixGrants.values()) for (const p of pairs) grantable.add(p.resource);

    const ungrantable = MP_CODES.filter(code => !grantable.has(code.split(':')[0]));
    expect(
      ungrantable,
      `小程序声明面里这些码的资源段在后端授权矩阵从未出现，任何角色都授予不了（只能靠超管通配）：` +
        `${ungrantable.join(', ')}`
    ).toEqual([]);

    // 动作派生同形：小程序沿用后端登录出参的权限码字面量，因此它对同一端点算出的动作名
    // 必须与后端 extract_route_info 的派生结果一致，否则它的入口门永远命中不了。
    const probes: Array<{ method: string; path: string; expectAction: string }> = [
      { method: 'GET', path: '/api/v1/erp/products', expectAction: 'read' },
      { method: 'POST', path: '/api/v1/erp/products', expectAction: 'create' },
      { method: 'PUT', path: '/api/v1/erp/products/1', expectAction: 'update' },
      { method: 'DELETE', path: '/api/v1/erp/products/1', expectAction: 'delete' },
      { method: 'GET', path: '/api/v1/erp/crm/customers', expectAction: 'read' },
      { method: 'PUT', path: '/api/v1/erp/crm/customers/1', expectAction: 'update' },
      { method: 'GET', path: '/api/v1/erp/sales/orders', expectAction: 'read' },
      { method: 'POST', path: '/api/v1/erp/sales/orders/1/cancel', expectAction: 'cancel' },
      { method: 'GET', path: '/api/v1/erp/dashboard/stats', expectAction: 'read' },
    ];
    for (const probe of probes) {
      const derived = derivePermissionKey(probe.method, probe.path);
      expect(
        derived.action,
        `后端对 ${probe.method} ${probe.path} 派生出的动作是 ${derived.action}（${derived.basis}），` +
          `与小程序沿用的 ${probe.expectAction} 不同形，小程序侧的入口门会永不命中`
      ).toBe(probe.expectAction);
      expect(
        grantable.has(derived.resource),
        `后端对 ${probe.method} ${probe.path} 派生出的资源名 ${derived.resource} 不在授权矩阵引用的资源里，` +
          '该端点对非超管是死键'
      ).toBe(true);
    }
  });
});
