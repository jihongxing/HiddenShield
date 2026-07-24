# Auth OTP 短信 / 邮件供应商生产接入 Checklist

本文档用于把当前 `auth/challenges` 的研发期 webhook 投递能力推进到真实短信 / 邮件供应商生产接入。当前代码已支持可配置 OTP delivery webhook；未配置时仍为 fixture，不得对用户承诺真实短信 / 邮件送达。

## 1. 当前已具备

- 后端正式 Auth API：`auth/challenges -> auth/sessions -> auth/refresh -> auth/logout -> me`。
- 验证码动态生成、hash / salt 存储、过期与 consumed 状态。
- 验证码发送限流与登录失败限流。
- `HIDDENSHIELD_AUTH_OTP_DELIVERY_ENDPOINT` 配置后，后端会向外部投递服务发送验证码，且不会向客户端返回 `fixtureCode`。
- 合同测试已覆盖 webhook 收到动态验证码并完成登录。

## 2. 生产接入阻断项

| 项目 | 当前状态 | 上线要求 |
| --- | --- | --- |
| 短信供应商 | 未选择 | 明确供应商、签名、签约主体、费率、国内/国际覆盖 |
| 邮件供应商 | 未选择 | 明确供应商、发信域名、SPF/DKIM/DMARC、退信处理 |
| 生产凭证 | 未配置 | 凭证只能在后端密钥系统保存，不能进入客户端或仓库 |
| 模板 ID | 未配置 | 短信和邮件模板均需审核通过后记录模板 ID |
| 模板内容 | 未冻结 | 需固定变量、过期时间、品牌名和反钓鱼提示 |
| 回调 / 告警 | 未配置 | 送达、失败、退信、限流、供应商异常需要进入监控 |
| 安全签名 | 待补强 | HiddenShield 后端到投递服务之间需加 HMAC 或等价签名 |
| 灰度验收 | 未执行 | 生产环境至少覆盖成功、限流、错误码、超时、重试和审计日志 |

## 3. 建议模板

短信模板：

```text
【HiddenShield】验证码：{code}，用于登录 HiddenShield 账户，{minutes} 分钟内有效。请勿转发给他人。
```

邮件主题：

```text
HiddenShield 登录验证码
```

邮件正文：

```text
你的 HiddenShield 登录验证码是：{code}

验证码将在 {minutes} 分钟后失效。若这不是你本人操作，请忽略本邮件。
```

## 4. 环境变量建议

保留当前后端抽象：

```text
HIDDENSHIELD_AUTH_OTP_DELIVERY_ENDPOINT=https://<otp-delivery-service>/v1/auth/otp
HIDDENSHIELD_AUTH_OTP_DELIVERY_CHANNEL=email_or_sms
```

新增生产安全建议：

```text
HIDDENSHIELD_AUTH_OTP_DELIVERY_SIGNING_SECRET=<secret-from-secret-manager>
HIDDENSHIELD_AUTH_OTP_SMS_PROVIDER=<provider>
HIDDENSHIELD_AUTH_OTP_SMS_TEMPLATE_ID=<approved-template-id>
HIDDENSHIELD_AUTH_OTP_EMAIL_PROVIDER=<provider>
HIDDENSHIELD_AUTH_OTP_EMAIL_TEMPLATE_ID=<approved-template-id>
```

## 5. 验收清单

- [ ] 短信模板审核通过并记录模板 ID。
- [ ] 邮件发信域名通过 SPF / DKIM / DMARC。
- [ ] 生产投递服务只接收后端签名请求。
- [ ] `auth/challenges` 配置生产 endpoint 后不返回 `fixtureCode`。
- [ ] 验证码可以通过短信完成登录。
- [ ] 验证码可以通过邮件完成登录。
- [ ] 供应商 4xx / 5xx / timeout 不落明文验证码日志。
- [ ] 同一账号、同一设备、同一 IP 的发送限流生效。
- [ ] 送达率、失败率、延迟和供应商异常进入告警。
- [ ] 运行态截图记录桌面端和移动端验证码发送、登录成功、限流提示、供应商失败提示。

## 6. 下一步

先选择真实短信和邮件供应商，并提供生产 endpoint、凭证存放方式、短信签名、邮件发信域名、模板 ID 和模板审核截图；随后再补后端 delivery 签名、供应商适配服务和生产联调证据。

