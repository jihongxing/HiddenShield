import 'package:flutter/material.dart';

import '../../app/mobile_app_state.dart';
import '../../shared/theme/design_tokens.dart';
import '../../shared/widgets/tool_cards.dart';

class OnboardingPage extends StatefulWidget {
  const OnboardingPage({super.key, required this.appState});

  final MobileAppState appState;

  @override
  State<OnboardingPage> createState() => _OnboardingPageState();
}

class _OnboardingPageState extends State<OnboardingPage> {
  late final TextEditingController _accountController = TextEditingController(
    text: widget.appState.syncProfile.accountLabel ?? '',
  );
  late final TextEditingController _passwordController =
      TextEditingController();
  late final TextEditingController _verificationCodeController =
      TextEditingController();
  late final TextEditingController _creatorController = TextEditingController(
    text: widget.appState.creatorLabel == '本机创作者'
        ? ''
        : widget.appState.creatorLabel,
  );
  _OnboardingStep _step = _OnboardingStep.account;
  String _signedInAccountLabel = '';
  String? _challengeId;
  String _authMode = 'code';
  bool _saveLocationConfirmed = false;
  bool _busy = false;
  bool _sendingCode = false;
  String _message = '';

  @override
  void dispose() {
    _accountController.dispose();
    _passwordController.dispose();
    _verificationCodeController.dispose();
    _creatorController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final canContinue =
        (_step == _OnboardingStep.account
            ? _accountController.text.trim().isNotEmpty &&
                  (_authMode == 'password'
                      ? _passwordController.text.isNotEmpty
                      : _challengeId != null &&
                            _verificationCodeController.text
                                .trim()
                                .isNotEmpty)
            : _creatorController.text.trim().isNotEmpty &&
                  _saveLocationConfirmed) &&
        !_busy;

    return Scaffold(
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.fromLTRB(18, 18, 18, 24),
          children: [
            _OnboardingHeader(step: _step),
            const SizedBox(height: HsSpacing.xl),
            AnimatedSwitcher(
              duration: const Duration(milliseconds: 180),
              child: _step == _OnboardingStep.account
                  ? _AccountStep(
                      key: const ValueKey('account'),
                      accountController: _accountController,
                      passwordController: _passwordController,
                      verificationCodeController: _verificationCodeController,
                      authMode: _authMode,
                      sendingCode: _sendingCode,
                      onModeChanged: (value) =>
                          setState(() => _authMode = value),
                      onSendCode: _sendCode,
                      onChanged: () => setState(() {}),
                    )
                  : _BaseSetupStep(
                      key: const ValueKey('setup'),
                      accountLabel: _signedInAccountLabel,
                      creatorController: _creatorController,
                      saveLocationConfirmed: _saveLocationConfirmed,
                      busy: _busy,
                      onCreatorChanged: () => setState(() {}),
                      onSaveLocationChanged: (value) => setState(
                        () => _saveLocationConfirmed = value ?? false,
                      ),
                      onChangeAccount: () {
                        widget.appState.signOutCloud();
                        setState(() {
                          _signedInAccountLabel = '';
                          _step = _OnboardingStep.account;
                        });
                      },
                    ),
            ),
            if (_message.isNotEmpty) ...[
              const SizedBox(height: HsSpacing.md),
              HsMessageCard(
                icon: Icons.info_outline,
                title: '无法继续',
                detail: _message,
                iconColor: HsColors.warning,
              ),
            ],
            const SizedBox(height: HsSpacing.xl),
            FilledButton.icon(
              onPressed: canContinue ? _continue : null,
              icon: _busy
                  ? const SizedBox(
                      width: 18,
                      height: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.arrow_forward_outlined),
              label: Text(
                _busy
                    ? '正在准备'
                    : _step == _OnboardingStep.account
                    ? '继续'
                    : '完成设置',
              ),
            ),
            if (_step == _OnboardingStep.account) ...[
              const SizedBox(height: HsSpacing.md),
              OutlinedButton.icon(
                onPressed: _busy ? null : _continueLocalOnly,
                icon: const Icon(Icons.phone_android_outlined),
                label: const Text('仅使用本机'),
              ),
              const SizedBox(height: HsSpacing.xs),
              Text(
                '无需账户即可完成本地创作者身份设置；云同步和所有云能力保持关闭。',
                textAlign: TextAlign.center,
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: HsColors.textMuted),
              ),
            ],
          ],
        ),
      ),
    );
  }

  void _continueLocalOnly() {
    setState(() {
      _signedInAccountLabel = '仅本机';
      _message = '';
      _step = _OnboardingStep.setup;
    });
  }

  Future<void> _continue() async {
    if (_step == _OnboardingStep.account) {
      setState(() {
        _busy = true;
        _message = '';
      });
      try {
        final signedIn = await widget.appState.continueWithAccountPlaceholder(
          accountLabel: _accountController.text,
          password: _authMode == 'password' ? _passwordController.text : '',
          challengeId: _authMode == 'code' ? _challengeId : null,
          verificationCode: _authMode == 'code'
              ? _verificationCodeController.text
              : '',
        );
        if (!signedIn) {
          if (mounted) {
            setState(
              () => _message =
                  widget.appState.syncProfile.lastError ?? '登录失败，请稍后重试',
            );
          }
          return;
        }
        if (!mounted) return;
        if (widget.appState.onboardingCompleted) {
          return;
        }
        setState(() {
          _signedInAccountLabel =
              widget.appState.syncProfile.accountLabel ??
              _accountController.text.trim();
          _step = _OnboardingStep.setup;
        });
      } catch (error) {
        if (mounted) {
          setState(() => _message = '$error');
        }
      } finally {
        if (mounted) {
          setState(() => _busy = false);
        }
      }
      return;
    }
    setState(() {
      _busy = true;
      _message = '';
    });
    try {
      await widget.appState.completeBaseSetup(
        creatorLabel: _creatorController.text,
      );
    } catch (error) {
      setState(() => _message = '$error');
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _sendCode() async {
    setState(() {
      _sendingCode = true;
      _message = '';
    });
    final challenge = await widget.appState.createAuthChallenge(
      accountLabel: _accountController.text,
    );
    if (!mounted) return;
    setState(() {
      _sendingCode = false;
      if (challenge != null) {
        _challengeId = challenge.challengeId;
        _verificationCodeController.text = challenge.fixtureCode ?? '';
        _message = challenge.fixtureCode == null ? challenge.message : '';
      } else {
        _message = widget.appState.syncProfile.lastError ?? '验证码发送失败';
      }
    });
  }
}

enum _OnboardingStep { account, setup }

class _OnboardingHeader extends StatelessWidget {
  const _OnboardingHeader({required this.step});

  final _OnboardingStep step;

  @override
  Widget build(BuildContext context) {
    return HsPanel(
      color: HsColors.surfaceRaised,
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                width: 42,
                height: 42,
                decoration: BoxDecoration(
                  color: HsColors.copper,
                  borderRadius: BorderRadius.circular(HsRadii.card),
                ),
                child: const Icon(
                  Icons.shield_outlined,
                  color: HsColors.background,
                ),
              ),
              const SizedBox(width: HsSpacing.md),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'HiddenShield',
                      style: Theme.of(context).textTheme.titleLarge?.copyWith(
                        fontWeight: FontWeight.w800,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      '本地优先的版权保护工具',
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: HsColors.textMuted,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: HsSpacing.xl),
          Text(
            step == _OnboardingStep.account ? '登录账户' : '基础设置',
            style: Theme.of(
              context,
            ).textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w800),
          ),
          const SizedBox(height: HsSpacing.xs),
          Text(
            step == _OnboardingStep.account
                ? '使用验证码或密码登录，也可以稍后只做本地保护。'
                : '设置写入版权记录的创作者身份。',
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: HsColors.textMuted,
              height: 1.35,
            ),
          ),
        ],
      ),
    );
  }
}

class _AccountStep extends StatelessWidget {
  const _AccountStep({
    super.key,
    required this.accountController,
    required this.passwordController,
    required this.verificationCodeController,
    required this.authMode,
    required this.sendingCode,
    required this.onModeChanged,
    required this.onSendCode,
    required this.onChanged,
  });

  final TextEditingController accountController;
  final TextEditingController passwordController;
  final TextEditingController verificationCodeController;
  final String authMode;
  final bool sendingCode;
  final ValueChanged<String> onModeChanged;
  final VoidCallback onSendCode;
  final VoidCallback onChanged;

  @override
  Widget build(BuildContext context) {
    return HsPanel(
      icon: Icons.account_circle_outlined,
      child: Column(
        children: [
          TextField(
            controller: accountController,
            keyboardType: TextInputType.emailAddress,
            decoration: const InputDecoration(
              labelText: '账号',
              hintText: '邮箱 / 手机号',
            ),
            onChanged: (_) => onChanged(),
          ),
          const SizedBox(height: HsSpacing.md),
          SegmentedButton<String>(
            segments: const [
              ButtonSegment(
                value: 'code',
                label: Text('验证码'),
                icon: Icon(Icons.pin_outlined),
              ),
              ButtonSegment(
                value: 'password',
                label: Text('密码'),
                icon: Icon(Icons.password_outlined),
              ),
            ],
            selected: {authMode},
            onSelectionChanged: (value) => onModeChanged(value.first),
          ),
          const SizedBox(height: HsSpacing.md),
          if (authMode == 'password')
            TextField(
              controller: passwordController,
              obscureText: true,
              decoration: const InputDecoration(
                labelText: '密码',
                hintText: '账户密码',
              ),
              onChanged: (_) => onChanged(),
            )
          else
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(
                  child: TextField(
                    controller: verificationCodeController,
                    keyboardType: TextInputType.number,
                    decoration: const InputDecoration(
                      labelText: '验证码',
                      hintText: '6 位验证码',
                    ),
                    onChanged: (_) => onChanged(),
                  ),
                ),
                const SizedBox(width: 8),
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: SizedBox(
                    width: 88,
                    child: OutlinedButton(
                      onPressed: sendingCode ? null : onSendCode,
                      child: Text(sendingCode ? '发送中' : '发送'),
                    ),
                  ),
                ),
              ],
            ),
        ],
      ),
    );
  }
}

class _BaseSetupStep extends StatelessWidget {
  const _BaseSetupStep({
    super.key,
    required this.accountLabel,
    required this.creatorController,
    required this.saveLocationConfirmed,
    required this.busy,
    required this.onCreatorChanged,
    required this.onSaveLocationChanged,
    required this.onChangeAccount,
  });

  final String accountLabel;
  final TextEditingController creatorController;
  final bool saveLocationConfirmed;
  final bool busy;
  final VoidCallback onCreatorChanged;
  final ValueChanged<bool?> onSaveLocationChanged;
  final VoidCallback onChangeAccount;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        HsPanel(
          icon: Icons.verified_user_outlined,
          child: Row(
            children: [
              Expanded(
                child: Text(
                  accountLabel,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(
                    context,
                  ).textTheme.bodyLarge?.copyWith(fontWeight: FontWeight.w700),
                ),
              ),
              TextButton(
                onPressed: busy ? null : onChangeAccount,
                child: const Text('更换'),
              ),
            ],
          ),
        ),
        const SizedBox(height: HsSpacing.md),
        HsPanel(
          icon: Icons.badge_outlined,
          child: TextField(
            controller: creatorController,
            decoration: const InputDecoration(
              labelText: '创作者身份',
              hintText: '工作室 / 艺名 / 公司名称',
            ),
            onChanged: (_) => onCreatorChanged(),
          ),
        ),
        const SizedBox(height: HsSpacing.md),
        HsPanel(
          icon: Icons.ios_share_outlined,
          child: CheckboxListTile(
            value: saveLocationConfirmed,
            onChanged: onSaveLocationChanged,
            contentPadding: EdgeInsets.zero,
            controlAffinity: ListTileControlAffinity.leading,
            title: const Text('由系统保存或分享保护副本'),
            subtitle: const Text('不默认同步原始媒体、保护副本或本地路径。'),
          ),
        ),
      ],
    );
  }
}
